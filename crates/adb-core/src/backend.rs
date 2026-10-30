use crate::{
    apk::{self, ApkInfo},
    identity, install_succeeded, shell_quote, validate_endpoint, validate_pairing_code,
};
use libadb::{
    Connection, Feature, TokioTcp,
    keys::{AdbKey, rsa::rand_core::OsRng},
    tls::{TlsClientConfig, TlsIdentity},
    transport::tls::MaybeTls,
};
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::{io::AsyncReadExt, time::timeout};
use uuid::Uuid;

type Wire = Connection<MaybeTls<TokioTcp>>;
#[derive(Debug, Clone, Serialize, thiserror::Error)]
#[serde(rename_all = "camelCase")]
#[error("{message}")]
pub struct PocError {
    pub kind: String,
    pub message: String,
}
impl PocError {
    pub fn new(kind: &str, message: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
        }
    }
}
type Result<T> = std::result::Result<T, PocError>;
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub id: String,
    pub device_id: String,
    pub profile_id: u32,
    pub serial: String,
    pub build: String,
    pub manufacturer: String,
    pub endpoint: String,
    pub model: String,
    pub android_version: String,
    pub sdk: u32,
    pub abis: Vec<String>,
    pub encrypted: bool,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub stage: String,
    pub sent: u64,
    pub total: u64,
}
struct Active {
    wire: Wire,
    info: DeviceInfo,
}
pub struct Client {
    key_dir: PathBuf,
    active: Option<Active>,
    label_cache: Option<(String, std::time::Instant, String)>,
}
impl Client {
    pub fn new(key_dir: PathBuf) -> Self {
        Self {
            key_dir,
            active: None,
            label_cache: None,
        }
    }
    pub fn status(&self) -> Option<DeviceInfo> {
        self.active.as_ref().map(|a| a.info.clone())
    }
    pub fn disconnect(&mut self) {
        self.active = None;
    }
    async fn identity(&self) -> Result<(AdbKey, TlsClientConfig)> {
        let dir = self.key_dir.clone();
        tokio::task::spawn_blocking(move || {
            let key = identity::load_or_create(&dir).map_err(|e| PocError::new("identity", e))?;
            let cert = TlsIdentity::from_key(&key, &mut OsRng)
                .map_err(|e| PocError::new("identity", e.to_string()))?;
            let tls = TlsClientConfig::adb(&cert)
                .map_err(|e| PocError::new("identity", e.to_string()))?;
            Ok((key, tls))
        })
        .await
        .map_err(|_| PocError::new("identity", "Could not prepare this app's host identity."))?
    }
    pub async fn pair(&mut self, endpoint: String, code: String) -> Result<String> {
        let endpoint = validate_endpoint(&endpoint).map_err(|e| PocError::new("input", e))?;
        validate_pairing_code(&code).map_err(|e| PocError::new("input", e))?;
        let (key, tls) = self.identity().await?;
        timeout(Duration::from_secs(35),async {
            let socket=tokio::net::TcpStream::connect(&endpoint).await.map_err(connection_error)?;
            let mut wire=MaybeTls::plain(TokioTcp::new(socket));
            libadb::pairing::pair(&mut wire,&tls,&key,&code,&mut OsRng).await.map(|v|v.guid).map_err(|e|PocError::new("pairing",format!("Pairing failed: {e}. Check the pairing address, code and open TV pairing screen.")))
        }).await.map_err(|_|PocError::new("timeout","Pairing timed out. Reopen the TV pairing screen and check its current port."))?
    }
    pub async fn connect(&mut self, endpoint: String, allow_legacy: bool) -> Result<DeviceInfo> {
        let endpoint = validate_endpoint(&endpoint).map_err(|e| PocError::new("input", e))?;
        self.active = None;
        let (key, tls) = self.identity().await?;
        let result=timeout(Duration::from_secs(25),async {
            let socket=tokio::net::TcpStream::connect(&endpoint).await.map_err(connection_error)?;
            socket.set_nodelay(true).map_err(connection_error)?;
            let mut wire=Connection::connect_tls(MaybeTls::plain(TokioTcp::new(socket)),key,&[Feature::ShellV2,Feature::StatV2,Feature::SendrecvV2],&tls).await.map_err(connection_error)?;
            let encrypted=wire.transport().is_tls();
            if !encrypted&&!allow_legacy{return Err(PocError::new("legacyApproval","This port uses unencrypted legacy ADB. Enable the legacy option only on a trusted local network."));}
            let output=command(&mut wire,"getprop ro.product.model; getprop ro.build.version.release; getprop ro.build.version.sdk; getprop ro.product.cpu.abilist; getprop ro.serialno; getprop ro.build.fingerprint; getprop ro.product.manufacturer; getprop persist.adb.wifi.guid").await?;
            if output.0!=Some(0){return Err(PocError::new("connection","Connected but device inspection failed."));}
            let lines:Vec<_>=output.1.lines().map(str::trim).collect();
            let field=|n:usize|lines.get(n).copied().unwrap_or("").to_owned();
            let id=Uuid::new_v4().to_string();let serial=field(4);let manufacturer=field(6);let guid=field(7);
            let device_id=crate::workspace::driver::device_identity(if guid.is_empty(){&serial}else{&guid},&manufacturer,&id);
            let info=DeviceInfo{id,device_id,profile_id:0,serial,manufacturer,build:field(5),endpoint:endpoint.clone(),model:lines.first().filter(|v|!v.is_empty()).unwrap_or(&"Android device").to_string(),android_version:field(1),sdk:field(2).parse().unwrap_or(0),abis:field(3).split(',').filter(|v|!v.is_empty()).map(str::to_owned).collect(),encrypted};
            Ok(Active{wire,info})
        }).await.map_err(|_|PocError::new("timeout","Connection timed out. Check the TV's connection port, local-network permission and approval prompt."))??;
        let info = result.info.clone();
        self.active = Some(result);
        Ok(info)
    }
    pub fn current_target(&self) -> Option<crate::workspace::types::Target> {
        self.active
            .as_ref()
            .map(|a| crate::workspace::types::Target {
                device_id: a.info.device_id.clone(),
                session_id: a.info.id.clone(),
                profile_id: a.info.profile_id,
            })
    }
    pub fn select_profile(
        &mut self,
        target: &crate::workspace::types::Target,
        profile_id: u32,
    ) -> Result<DeviceInfo> {
        crate::workspace::types::validate_target(self.current_target().as_ref(), target)?;
        let active = self
            .active
            .as_mut()
            .ok_or_else(|| PocError::new("staleTarget", "Device disconnected."))?;
        if active.info.profile_id != profile_id {
            active.info.profile_id = profile_id;
            active.info.id = Uuid::new_v4().to_string();
        }
        Ok(active.info.clone())
    }
    pub async fn execute_target(
        &mut self,
        target: &crate::workspace::types::Target,
        cmd: &str,
    ) -> Result<crate::workspace::driver::ShellOutput> {
        crate::workspace::types::validate_target(self.current_target().as_ref(), target)?;
        if cmd.contains('\0') || cmd.len() > 65536 {
            return Err(PocError::new(
                "input",
                "Command is invalid or exceeds 64 KiB.",
            ));
        }
        let mut active = self
            .active
            .take()
            .ok_or_else(|| PocError::new("staleTarget", "Device disconnected."))?;
        match timeout(
            Duration::from_secs(30),
            command_output(&mut active.wire, cmd),
        )
        .await
        {
            Ok(Ok(output)) => {
                self.active = Some(active);
                Ok(output)
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(PocError::new(
                "timeout",
                "Device operation timed out. Reconnect before retrying.",
            )),
        }
    }
    pub async fn inspect_metadata(
        &mut self,
        target: &crate::workspace::types::Target,
        packages: &[String],
    ) -> Result<Vec<crate::workspace::metadata::Metadata>> {
        crate::workspace::types::validate_target(self.current_target().as_ref(), target)?;
        if packages.is_empty() {
            return Ok(Vec::new());
        }
        if packages.len() > 32 {
            return Err(PocError::new(
                "input",
                "Inspect at most 32 packages at once.",
            ));
        }
        let arguments = packages
            .iter()
            .map(|p| crate::workspace::commands::package_arg(p))
            .collect::<Result<Vec<_>>>()?
            .join(" ");
        let output = self.run_inspector(target, &arguments).await?;
        crate::workspace::metadata::parse_metadata(&output, packages)
    }
    async fn run_inspector(
        &mut self,
        target: &crate::workspace::types::Target,
        arguments: &str,
    ) -> Result<String> {
        crate::workspace::types::validate_target(self.current_target().as_ref(), target)?;
        const JAR: &[u8] = include_bytes!("../assets/bridgecoil-inspector.jar");
        use sha2::Digest;
        let hash = hex::encode(sha2::Sha256::digest(JAR));
        let remote = format!("/data/local/tmp/bridgecoil-inspector-{hash}.jar");
        let mut active = self
            .active
            .take()
            .ok_or_else(|| PocError::new("connection", "Device disconnected."))?;
        let staged = timeout(Duration::from_secs(15), async {
            let mut buffer = vec![0u8; 66000];
            let mut sync = libadb::sync::open(&mut active.wire, &mut buffer)
                .await
                .map_err(connection_error)?;
            sync.send_start(&remote, 0o100600)
                .await
                .map_err(connection_error)?;
            for chunk in JAR.chunks(65536) {
                sync.send_data(chunk).await.map_err(connection_error)?;
            }
            sync.send_done(0).await.map_err(connection_error)?;
            sync.quit().await.map_err(connection_error)
        })
        .await
        .map_err(|_| {
            PocError::new(
                "timeout",
                "App metadata staging timed out. Reconnect to continue.",
            )
        })?;
        staged?;
        self.active = Some(active);
        let output=self.execute_target(target,&format!("CLASSPATH={} app_process /system/bin nl.neerdael.bridgecoil.BridgecoilInspector {} {}",shell_quote(&remote),target.profile_id,arguments)).await?;
        if !output.success() {
            return Err(PocError::new("metadata", output.combined()));
        }
        Ok(output.stdout)
    }
    pub async fn install(
        &mut self,
        session: &str,
        path: &Path,
        expected: &ApkInfo,
        mut progress: impl FnMut(InstallProgress) + Send,
    ) -> Result<()> {
        if self.active.as_ref().is_none_or(|a| a.info.id != session) {
            return Err(PocError::new(
                "staleSession",
                "The connection changed. Reconnect and choose the APK again.",
            ));
        }
        let path = path.to_path_buf();
        let checked_path = path.clone();
        let actual = tokio::task::spawn_blocking(move || apk::inspect(&checked_path))
            .await
            .map_err(|_| PocError::new("apk", "Could not inspect selected APK."))?
            .map_err(|e| PocError::new("apk", e))?;
        if actual.sha256 != expected.sha256
            || actual.package != expected.package
            || actual.version_code != expected.version_code
        {
            return Err(PocError::new(
                "apkChanged",
                "The selected APK changed. Select and review it again.",
            ));
        }
        let mut active = self
            .active
            .take()
            .ok_or_else(|| PocError::new("staleSession", "Device disconnected."))?;
        if actual
            .min_sdk
            .is_some_and(|min| active.info.sdk != 0 && min > active.info.sdk)
        {
            self.active = Some(active);
            return Err(PocError::new(
                "incompatible",
                "APK requires a newer Android version than this TV.",
            ));
        }
        if !actual.abis.is_empty()
            && !active.info.abis.is_empty()
            && !actual.abis.iter().any(|v| active.info.abis.contains(v))
        {
            self.active = Some(active);
            return Err(PocError::new(
                "incompatible",
                "APK contains no native libraries compatible with this TV.",
            ));
        }
        let remote = format!("/data/local/tmp/adb-studio-{}.apk", Uuid::new_v4());
        let mut installer_started = false;
        let result=timeout(Duration::from_secs(300),async {
            progress(InstallProgress{stage:"uploading".into(),sent:0,total:actual.bytes});
            {
                let mut buffer=vec![0u8;66000];
                let mut sync=libadb::sync::open(&mut active.wire,&mut buffer).await.map_err(connection_error)?;
                sync.send_start(&remote,0o100600).await.map_err(connection_error)?;
                let mut file=tokio::fs::File::open(&path).await.map_err(connection_error)?;
                let mut chunk=vec![0u8;65536];let mut sent=0;
                loop {let n=file.read(&mut chunk).await.map_err(connection_error)?;if n==0{break;}sync.send_data(&chunk[..n]).await.map_err(connection_error)?;sent+=n as u64;progress(InstallProgress{stage:"uploading".into(),sent,total:actual.bytes});}
                if sent!=actual.bytes{return Err(PocError::new("apkChanged","APK size changed during transfer."));}
                sync.send_done(0).await.map_err(connection_error)?;
                sync.quit().await.map_err(connection_error)?;
            }
            progress(InstallProgress{stage:"installing".into(),sent:actual.bytes,total:actual.bytes});
            installer_started=true;
            let output=command(&mut active.wire,&format!("pm install -r --user {} {}",active.info.profile_id,shell_quote(&remote))).await?;
            if !install_succeeded(output.0,&output.1) {return Err(PocError::new("installRejected",format!("Android rejected the installation: {}",output.1.trim())));}
            progress(InstallProgress{stage:"verifying".into(),sent:actual.bytes,total:actual.bytes});
            let output=command(&mut active.wire,&format!("dumpsys package {}",shell_quote(&actual.package))).await?;
            if output.0!=Some(0)||installed_version(&output.1)!=Some(actual.version_code){return Err(PocError::new("unknownInstall","Android reported success, but the expected installed version could not be verified. Check the TV before retrying."));}
            Ok(())
        }).await;
        // Best effort, bounded cleanup of only our unpredictable staging path. Never retry the install.
        let _ = timeout(
            Duration::from_secs(5),
            command(&mut active.wire, &format!("rm -f {}", shell_quote(&remote))),
        )
        .await;
        match result {
            Ok(Ok(())) => {
                self.active = Some(active);
                Ok(())
            }
            Ok(Err(e)) if e.kind == "installRejected" => {
                self.active = Some(active);
                Err(e)
            }
            Ok(Err(e)) if !installer_started => Err(e),
            Ok(Err(e)) => Err(PocError::new(
                "unknownInstall",
                format!(
                    "Installation outcome is not verified. Reconnect and inspect the TV before retrying. {}",
                    e.message
                ),
            )),
            Err(_) => Err(PocError::new(
                if installer_started {
                    "unknownInstall"
                } else {
                    "timeout"
                },
                "Operation timed out. Reconnect and check the TV before retrying.",
            )),
        }
    }
}
fn connection_error(e: impl std::fmt::Display) -> PocError {
    PocError::new("connection", e.to_string())
}
pub fn installed_version(output: &str) -> Option<u64> {
    output.lines().find_map(|l| {
        l.trim()
            .strip_prefix("versionCode=")
            .and_then(|v| v.split_whitespace().next())
            .and_then(|v| v.parse().ok())
    })
}
async fn command(wire: &mut Wire, cmd: &str) -> Result<(Option<u8>, String)> {
    let output = command_output(wire, cmd).await?;
    Ok((output.exit_code, output.combined()))
}
async fn command_output(
    wire: &mut Wire,
    cmd: &str,
) -> Result<crate::workspace::driver::ShellOutput> {
    use libadb::shell::v2::{self, Frame};
    let mut rx = vec![0u8; 65536];
    let mut shell = v2::open(wire, cmd, &mut rx)
        .await
        .map_err(connection_error)?;
    shell.close_stdin().await.map_err(connection_error)?;
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit = loop {
        match shell.read_frame().await.map_err(connection_error)? {
            Frame::Stdout(b) => {
                if stdout.len() + stderr.len() + b.len() > 1024 * 1024 {
                    return Err(PocError::new(
                        "outputLimit",
                        "Device output exceeded the 1 MiB limit.",
                    ));
                }
                stdout.extend_from_slice(&b);
            }
            Frame::Stderr(b) => {
                if stdout.len() + stderr.len() + b.len() > 1024 * 1024 {
                    return Err(PocError::new(
                        "outputLimit",
                        "Device output exceeded the 1 MiB limit.",
                    ));
                }
                stderr.extend_from_slice(&b);
            }
            Frame::Exit(code) => break code,
            _ => {}
        }
    };
    shell.close().await.map_err(connection_error)?;
    Ok(crate::workspace::driver::ShellOutput {
        exit_code: Some(exit),
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
    })
}
#[async_trait::async_trait]
impl crate::workspace::driver::ShellDriver for Client {
    async fn labels(
        &mut self,
        target: &crate::workspace::types::Target,
        packages: &[String],
    ) -> Result<Option<Vec<crate::workspace::metadata::Metadata>>> {
        crate::workspace::types::validate_target(self.current_target().as_ref(), target)?;
        if let Some((session, when, raw)) = &self.label_cache
            && session == &target.session_id
            && when.elapsed() < Duration::from_secs(30)
        {
            return crate::workspace::metadata::parse_metadata(raw, packages).map(Some);
        }
        let output = self.run_inspector(target, "--labels").await?;
        let rows = crate::workspace::metadata::parse_metadata(&output, packages)?;
        self.label_cache = Some((target.session_id.clone(), std::time::Instant::now(), output));
        Ok(Some(rows))
    }
    async fn metadata(
        &mut self,
        target: &crate::workspace::types::Target,
        packages: &[String],
    ) -> Result<Option<Vec<crate::workspace::metadata::Metadata>>> {
        self.inspect_metadata(target, packages).await.map(Some)
    }
    fn current_target(&self) -> Option<crate::workspace::types::Target> {
        Client::current_target(self)
    }
    async fn shell(
        &mut self,
        target: &crate::workspace::types::Target,
        command: &str,
    ) -> Result<crate::workspace::driver::ShellOutput> {
        self.execute_target(target, command).await
    }
}
