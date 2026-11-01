use crate::{
    apk::{self, ApkInfo},
    identity, install_succeeded, shell_quote, validate_endpoint, validate_pairing_code,
    workspace::files,
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
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::{io::AsyncReadExt, time::timeout};
use uuid::Uuid;

type Wire = Connection<MaybeTls<TokioTcp>>;
#[derive(Debug, Clone, Serialize, thiserror::Error)]
#[serde(rename_all = "camelCase")]
#[error("{message}")]
pub struct PocError {
    pub kind: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}
impl PocError {
    pub fn new(kind: &str, message: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
            details: None,
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
    /// What Wake-on-LAN needs, saved while the TV is awake enough to say.
    pub hardware_addresses: Vec<String>,
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
        self.connect_with_progress(endpoint, allow_legacy, |_| {})
            .await
    }
    pub async fn connect_with_progress<F>(
        &mut self,
        endpoint: String,
        allow_legacy: bool,
        progress: F,
    ) -> Result<DeviceInfo>
    where
        F: Fn(crate::connection::ConnectionProgress) + Send + Sync + 'static,
    {
        use crate::connection::{
            ConnectionAuth, ConnectionProgress, ConnectionStage, connection_failure,
        };
        use std::sync::{Arc, atomic::Ordering};
        let endpoint = validate_endpoint(&endpoint).map_err(|e| PocError::new("input", e))?;
        self.active = None;
        let progress = Arc::new(progress);
        progress(ConnectionProgress {
            stage: ConnectionStage::Preparing,
        });
        let (key, tls) = self.identity().await?;
        progress(ConnectionProgress {
            stage: ConnectionStage::Connecting,
        });
        let socket = timeout(
            Duration::from_secs(8),
            tokio::net::TcpStream::connect(&endpoint),
        )
        .await
        .map_err(|_| {
            PocError::new(
                "timeout",
                "The TV did not answer on this address. Check the current connection port.",
            )
        })?
        .map_err(connection_error)?;
        socket.set_nodelay(true).map_err(connection_error)?;
        progress(ConnectionProgress {
            stage: ConnectionStage::Negotiating,
        });
        let auth = ConnectionAuth::new(key, allow_legacy, progress.clone());
        let offered = auth.offered.clone();
        let mut wire = {
            let mut handshake = Box::pin(Connection::connect_tls(
                MaybeTls::plain(TokioTcp::new(socket)),
                auth,
                &[Feature::ShellV2, Feature::StatV2, Feature::SendrecvV2],
                &tls,
            ));
            let mut deadline = Box::pin(tokio::time::sleep(Duration::from_secs(25)));
            let mut extended = false;
            loop {
                tokio::select! {
                    result=&mut handshake=>break result.map_err(connection_failure)?,
                    _=&mut deadline=>{
                        if offered.load(Ordering::SeqCst)&&!extended {extended=true;deadline.as_mut().reset(tokio::time::Instant::now()+Duration::from_secs(155));}
                        else{return Err(PocError::new("timeout",if offered.load(Ordering::SeqCst){"TV authorization was not completed. Approve this app's debugging prompt on the TV, then try again."}else{"ADB did not finish its handshake. Check the connection port and pair this installation if using Wireless debugging."}));}
                    }
                }
            }
        };
        let encrypted = wire.transport().is_tls();
        if !encrypted && !allow_legacy {
            return Err(PocError::new(
                "legacyApproval",
                "This port uses unencrypted legacy ADB. Enable the legacy option only on a trusted local network.",
            ));
        }
        progress(ConnectionProgress {
            stage: ConnectionStage::Inspecting,
        });
        let output=timeout(Duration::from_secs(10),command(&mut wire,"getprop ro.product.model; getprop ro.build.version.release; getprop ro.build.version.sdk; getprop ro.product.cpu.abilist; getprop ro.serialno; getprop ro.build.fingerprint; getprop ro.product.manufacturer; getprop persist.adb.wifi.guid; for i in eth0 wlan0; do echo $i $(cat /sys/class/net/$i/address 2>/dev/null); done; ip -o link 2>/dev/null; true")).await.map_err(|_|PocError::new("timeout","ADB connected, but the TV's device inspection did not finish."))??;
        if output.0 != Some(0) {
            return Err(PocError::new(
                "connection",
                "Connected but device inspection failed.",
            ));
        }
        let lines: Vec<_> = output.1.lines().map(str::trim).collect();
        let field = |n: usize| lines.get(n).copied().unwrap_or("").to_owned();
        let id = Uuid::new_v4().to_string();
        let serial = field(4);
        let manufacturer = field(6);
        let guid = field(7);
        let device_id = crate::workspace::driver::device_identity(
            if guid.is_empty() { &serial } else { &guid },
            &manufacturer,
            &id,
        );
        let info = DeviceInfo {
            id,
            device_id,
            profile_id: 0,
            serial,
            manufacturer,
            build: field(5),
            endpoint: endpoint.clone(),
            model: lines
                .first()
                .filter(|v| !v.is_empty())
                .unwrap_or(&"Android device")
                .to_string(),
            android_version: field(1),
            sdk: field(2).parse().unwrap_or(0),
            abis: field(3)
                .split(',')
                .filter(|v| !v.is_empty())
                .map(str::to_owned)
                .collect(),
            encrypted,
            hardware_addresses: crate::wake::parse_hardware_addresses(
                lines.get(8..).unwrap_or_default(),
            ),
        };
        self.active = Some(Active {
            wire,
            info: info.clone(),
        });
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
        self.execute_limited(target, cmd, SHELL_OUTPUT_LIMIT, Overflow::Refuse)
            .await
    }
    async fn execute_limited(
        &mut self,
        target: &crate::workspace::types::Target,
        cmd: &str,
        limit: usize,
        overflow: Overflow,
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
            command_output_limited(&mut active.wire, cmd, limit, overflow),
        )
        .await
        {
            Ok(Ok(output)) => {
                self.active = Some(active);
                Ok(output)
            }
            Ok(Err(e)) => {
                if keeps_connection(&e) {
                    self.active = Some(active);
                }
                Err(e)
            }
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
    /// Puts the agent for this TV's CPU into Termux. It needs Termux to be set up first.
    pub async fn install_agent(&mut self, session: &str) -> Result<()> {
        let info = self.status().filter(|i| i.id == session).ok_or_else(|| {
            PocError::new("staleSession", "The connection changed. Reconnect and try again.")
        })?;
        let binary = crate::toolkit::agent::binary_for(&info.abis).ok_or_else(|| {
            PocError::new("unsupported", "Bridgecoil has no agent for this TV's processor.")
        })?;
        let remote = format!("/data/local/tmp/bridgecoil-agent-{}", Uuid::new_v4());
        let mut active = self
            .active
            .take()
            .ok_or_else(|| PocError::new("staleTarget", "Device disconnected."))?;
        let staged = timeout(Duration::from_secs(60), async {
            let mut buffer = vec![0u8; 66000];
            let mut sync = libadb::sync::open(&mut active.wire, &mut buffer)
                .await
                .map_err(connection_error)?;
            sync.send_start(&remote, 0o100644)
                .await
                .map_err(connection_error)?;
            for chunk in binary.chunks(65536) {
                sync.send_data(chunk).await.map_err(connection_error)?;
            }
            sync.send_done(0).await.map_err(connection_error)?;
            sync.quit().await.map_err(connection_error)
        })
        .await
        .map_err(|_| PocError::new("timeout", "Copying to the TV timed out. Reconnect and try again."))?;
        staged?;
        self.active = Some(active);
        let target = self
            .current_target()
            .ok_or_else(|| PocError::new("staleTarget", "Device disconnected."))?;
        let output = self
            .execute_target(&target, &crate::toolkit::agent::place_command(&remote))
            .await?;
        if !output.success() {
            return Err(PocError::new("agent", output.combined()));
        }
        Ok(())
    }
    /// What decides which parts of an app this TV needs. A TV that won't say still gets its app,
    /// only with more parts than it strictly needs.
    pub async fn install_profile(&mut self, session: &str) -> Result<crate::bundle::DeviceProfile> {
        let target = self
            .current_target()
            .filter(|t| t.session_id == session)
            .ok_or_else(|| {
                PocError::new("staleSession", "Connect to a TV before choosing a file.")
            })?;
        let info = self.status().ok_or_else(|| {
            PocError::new("staleSession", "Connect to a TV before choosing a file.")
        })?;
        let output = self
            .execute_target(&target, crate::bundle::PROFILE_COMMAND)
            .await?;
        Ok(crate::bundle::parse_profile(
            info.sdk,
            info.abis,
            &output.combined(),
        ))
    }
    /// Installs an app made of several APKs, as one session. Android takes all of it or none, so
    /// a failure can't leave half an app on the TV. Game data follows only once the app is proven
    /// to be there, since it means nothing without one and can be as large as the TV's free space.
    pub async fn install_set(
        &mut self,
        session: &str,
        set: &crate::bundle::Unpacked,
        mut progress: impl FnMut(InstallProgress) + Send,
    ) -> Result<()> {
        if self.active.as_ref().is_none_or(|a| a.info.id != session) {
            return Err(PocError::new(
                "staleSession",
                "The connection changed. Reconnect and choose the file again.",
            ));
        }
        let parts: Vec<_> = set
            .paths
            .iter()
            .cloned()
            .zip(set.set.files.iter().map(|f| f.info.sha256.clone()))
            .collect();
        tokio::task::spawn_blocking(move || -> Result<()> {
            for (path, sha256) in parts {
                let actual = apk::inspect_package(&path).map_err(|e| PocError::new("apk", e))?;
                if actual.sha256 != sha256 {
                    return Err(PocError::new(
                        "apkChanged",
                        "The selected file changed. Choose it and review it again.",
                    ));
                }
            }
            Ok(())
        })
        .await
        .map_err(|_| PocError::new("apk", "Could not check the selected file."))??;
        let mut active = self
            .active
            .take()
            .ok_or_else(|| PocError::new("staleSession", "Device disconnected."))?;
        let mut committing = false;
        let result = install_parts(&mut active, set, &mut committing, &mut progress).await;
        self.label_cache = None;
        match result {
            Ok(()) => {
                self.active = Some(active);
                Ok(())
            }
            Err(e) if ["installRejected", "gameData"].contains(&e.kind.as_str()) => {
                self.active = Some(active);
                Err(e)
            }
            Err(e) if !committing => Err(e),
            Err(e) => Err(PocError::new(
                "unknownInstall",
                format!(
                    "Installation outcome is not verified. Reconnect and inspect the TV before retrying. {}",
                    e.message
                ),
            )),
        }
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
            if !install_succeeded(output.0,&output.1) {return Err(PocError::new("installRejected",install_rejection(&output.1)));}
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
                self.label_cache = None;
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
    /// Streams `file` to a staging name and only then gives it its real one, so an
    /// interrupted upload can never look finished.
    pub async fn upload(
        &mut self,
        session: &str,
        mut file: tokio::fs::File,
        total: u64,
        remote: &str,
        cancel: &AtomicBool,
        mut progress: impl FnMut(u64) + Send,
    ) -> Result<()> {
        if self.active.as_ref().is_none_or(|a| a.info.id != session) {
            return Err(PocError::new(
                "staleSession",
                "The connection changed. Reconnect and choose the file again.",
            ));
        }
        let mut active = self
            .active
            .take()
            .ok_or_else(|| PocError::new("staleSession", "Device disconnected."))?;
        let quoted = shell_quote(remote);
        let staging = format!("{remote}.bridgecoil-part");
        let staged = shell_quote(&staging);
        let exists = command(
            &mut active.wire,
            &format!("[ -e {quoted} ] || [ -L {quoted} ]"),
        )
        .await;
        match exists {
            Ok((Some(0), _)) => {
                self.active = Some(active);
                return Err(PocError::new(
                    "exists",
                    "An item with this name already exists here.",
                ));
            }
            Ok(_) => {}
            Err(e) => return Err(files::after_disconnect(e)),
        }
        let transfer: Result<bool> = async {
            let mut buffer = vec![0u8; 66000];
            let mut sync = libadb::sync::open(&mut active.wire, &mut buffer)
                .await
                .map_err(connection_error)?;
            sync.send_start(&staging, 0o100644)
                .await
                .map_err(connection_error)?;
            let mut chunk = vec![0u8; 65536];
            let mut sent = 0;
            let mut finished = true;
            loop {
                if cancel.load(Ordering::SeqCst) {
                    finished = false;
                    break;
                }
                let n = file.read(&mut chunk).await.map_err(connection_error)?;
                if n == 0 {
                    break;
                }
                match files::interruptible(sync.send_data(&chunk[..n]), cancel, UPLOAD_STALL).await
                {
                    Ok(sent) => sent.map_err(connection_error)?,
                    Err(files::Interrupted::Cancelled) => {
                        return Err(PocError::new("cancelled", "Upload cancelled."));
                    }
                    Err(files::Interrupted::Stalled) => {
                        return Err(PocError::new("timeout", "The upload stalled."));
                    }
                }
                sent += n as u64;
                progress(sent);
            }
            sync.send_done(0).await.map_err(connection_error)?;
            sync.quit().await.map_err(connection_error)?;
            if finished && sent != total {
                return Err(PocError::new(
                    "fileChanged",
                    "The file changed size during the upload.",
                ));
            }
            Ok(finished)
        }
        .await;
        let verified = match &transfer {
            Ok(true) => command(
                &mut active.wire,
                &format!("mv -n {staged} {quoted} && stat -c %s {quoted}"),
            )
            .await
            .is_ok_and(|(code, output)| code == Some(0) && output.trim() == total.to_string()),
            _ => false,
        };
        if verified {
            self.active = Some(active);
            return Ok(());
        }
        // Best effort, bounded cleanup. A clean reply also proves the connection is still usable.
        let cleaned = timeout(
            Duration::from_secs(5),
            command(&mut active.wire, &format!("rm -f {staged}")),
        )
        .await;
        let failure = match transfer {
            Err(e) => files::upload_failure(e),
            Ok(false) => PocError::new("cancelled", "Upload cancelled."),
            Ok(true) => PocError::new(
                "unknownUpload",
                "The upload finished, but the file could not be verified on the TV.",
            ),
        };
        if matches!(cleaned, Ok(Ok(_))) {
            self.active = Some(active);
            return Err(failure);
        }
        Err(files::after_disconnect(failure))
    }
    /// Run before asking where to save, because choosing an existing file empties it.
    pub async fn can_read(&mut self, session: &str, remote: &str) -> Result<()> {
        let target = self
            .current_target()
            .filter(|_| self.active.as_ref().is_some_and(|a| a.info.id == session))
            .ok_or_else(|| {
                PocError::new(
                    "staleSession",
                    "The connection changed. Reconnect and try again.",
                )
            })?;
        let check = format!("[ -r {} ]", shell_quote(remote));
        if self.execute_target(&target, &check).await?.success() {
            return Ok(());
        }
        Err(PocError::new(
            "fileUnavailable",
            "This file can't be read over ADB.",
        ))
    }
    /// Copies a file off the TV. The write side is the caller's, because on a phone it is a
    /// document the user picked rather than a plain path.
    pub async fn download(
        &mut self,
        session: &str,
        remote: &str,
        sink: &mut (impl tokio::io::AsyncWrite + Unpin + Send),
        cancel: &AtomicBool,
        mut progress: impl FnMut(u64, u64) + Send,
    ) -> Result<u64> {
        use tokio::io::AsyncWriteExt;
        if self.active.as_ref().is_none_or(|a| a.info.id != session) {
            return Err(PocError::new(
                "staleSession",
                "The connection changed. Reconnect and try again.",
            ));
        }
        let mut active = self
            .active
            .take()
            .ok_or_else(|| PocError::new("staleSession", "Device disconnected."))?;
        let quoted = shell_quote(remote);
        let total = match command(&mut active.wire, &format!("stat -L -c %s {quoted}")).await {
            Ok((Some(0), output)) => output.trim().parse::<u64>().ok(),
            Ok(_) => None,
            Err(e) => return Err(files::after_disconnect(e)),
        };
        let Some(total) = total else {
            self.active = Some(active);
            return Err(PocError::new(
                "fileUnavailable",
                "This file can't be read over ADB.",
            ));
        };
        let transfer: Result<u64> = async {
            let mut buffer = vec![0u8; 66000];
            let mut sync = libadb::sync::open(&mut active.wire, &mut buffer)
                .await
                .map_err(connection_error)?;
            sync.recv_start(remote).await.map_err(connection_error)?;
            let mut received = 0;
            loop {
                let next = match files::interruptible(sync.recv_next(), cancel, UPLOAD_STALL).await
                {
                    Ok(next) => next.map_err(connection_error)?,
                    Err(files::Interrupted::Cancelled) => {
                        // Each abandoned channel would use up one of the connection's slots.
                        let _ = timeout(Duration::from_secs(2), sync.quit()).await;
                        return Err(PocError::new("cancelled", "Download cancelled."));
                    }
                    Err(files::Interrupted::Stalled) => {
                        return Err(PocError::new("timeout", "The download stalled."));
                    }
                };
                let Some(chunk) = next else { break };
                sink.write_all(chunk).await.map_err(connection_error)?;
                received += chunk.len() as u64;
                progress(received, total);
            }
            sink.flush().await.map_err(connection_error)?;
            sync.quit().await.map_err(connection_error)?;
            Ok(received)
        }
        .await;
        match transfer {
            Ok(received) if received == total => {
                self.active = Some(active);
                Ok(received)
            }
            Ok(_) => {
                self.active = Some(active);
                Err(PocError::new(
                    "fileChanged",
                    "The file changed while it was being copied.",
                ))
            }
            Err(failure) => {
                // A stopped transfer can leave data in flight, so only keep a connection that
                // still answers.
                let alive = timeout(
                    Duration::from_secs(5),
                    command(&mut active.wire, "echo ok"),
                )
                .await;
                if matches!(alive, Ok(Ok((Some(0), _)))) {
                    self.active = Some(active);
                    return Err(failure);
                }
                Err(files::after_disconnect(failure))
            }
        }
    }
}
const UPLOAD_STALL: Duration = Duration::from_secs(60);
async fn install_parts(
    active: &mut Active,
    set: &crate::bundle::Unpacked,
    committing: &mut bool,
    progress: &mut (impl FnMut(InstallProgress) + Send),
) -> Result<()> {
    let apks: u64 = set.set.files.iter().map(|f| f.info.bytes).sum();
    let data: u64 = set.obb.iter().map(|o| o.bytes).sum();
    let total = apks + data;
    let profile = active.info.profile_id;
    let wire = &mut active.wire;
    let rejected = |output: &str| PocError::new("installRejected", install_rejection(output));
    let created = command(wire, &format!("pm install-create -r --user {profile} -S {apks}")).await?;
    let session = install_session(created.1.trim()).ok_or_else(|| rejected(&created.1))?;
    let mut done = 0;
    let written: Result<()> = async {
        progress(InstallProgress { stage: "uploading".into(), sent: 0, total });
        for (index, (path, file)) in set.paths.iter().zip(&set.set.files).enumerate() {
            let remote = format!("/data/local/tmp/bridgecoil-{}.apk", Uuid::new_v4());
            let pushed = push_part(wire, path, &remote, 0o100600, &mut done, &mut |sent| {
                progress(InstallProgress { stage: "uploading".into(), sent, total })
            })
            .await;
            let output = match pushed {
                Ok(()) => {
                    command(
                        wire,
                        &format!(
                            "pm install-write -S {} {session} part{index}.apk {}",
                            file.info.bytes,
                            shell_quote(&remote)
                        ),
                    )
                    .await
                }
                Err(e) => Err(e),
            };
            // TVs have little space, so a part leaves as soon as Android holds its own copy.
            let _ = timeout(
                Duration::from_secs(5),
                command(wire, &format!("rm -f {}", shell_quote(&remote))),
            )
            .await;
            let output = output?;
            if output.0 != Some(0) || !output.1.trim().starts_with("Success") {
                return Err(rejected(&output.1));
            }
        }
        Ok(())
    }
    .await;
    if let Err(e) = written {
        let _ = timeout(
            Duration::from_secs(5),
            command(wire, &format!("pm install-abandon {session}")),
        )
        .await;
        return Err(e);
    }
    progress(InstallProgress { stage: "installing".into(), sent: done, total });
    *committing = true;
    // Play Protect can hold this open until someone answers it on the TV.
    let output = timeout(
        Duration::from_secs(300),
        command(wire, &format!("pm install-commit {session}")),
    )
    .await
    .map_err(|_| PocError::new("timeout", "Android didn't finish installing in time."))??;
    if !install_succeeded(output.0, &output.1) {
        return Err(rejected(&output.1));
    }
    progress(InstallProgress { stage: "verifying".into(), sent: done, total });
    let package = &set.set.package;
    let output = command(wire, &format!("dumpsys package {}", shell_quote(package))).await?;
    if output.0 != Some(0) || installed_version(&output.1) != Some(set.set.version_code) {
        return Err(PocError::new(
            "unknownInstall",
            "Android reported success, but the expected installed version could not be verified. Check the TV before retrying.",
        ));
    }
    if set.obb.is_empty() {
        return Ok(());
    }
    let failed = |why: String| {
        PocError::new(
            "gameData",
            format!("The app is installed, but its game data couldn't be copied. {why}"),
        )
    };
    let folder = format!("/storage/emulated/{profile}/Android/obb/{package}");
    let made = command(wire, &format!("mkdir -p {}", shell_quote(&folder)))
        .await
        .map_err(|e| failed(e.message))?;
    if made.0 != Some(0) {
        return Err(failed(made.1));
    }
    for obb in &set.obb {
        let target = format!("{folder}/{}", obb.name);
        let staging = format!("{target}.bridgecoil-part");
        push_part(wire, &obb.path, &staging, 0o100644, &mut done, &mut |sent| {
            progress(InstallProgress { stage: "copyingData".into(), sent, total })
        })
        .await
        .map_err(|e| failed(e.message))?;
        let moved = command(
            wire,
            &format!("mv -f {} {}", shell_quote(&staging), shell_quote(&target)),
        )
        .await
        .map_err(|e| failed(e.message))?;
        if moved.0 != Some(0) {
            return Err(failed(moved.1));
        }
    }
    Ok(())
}
/// Pushes one file, counting towards the install's overall progress.
async fn push_part(
    wire: &mut Wire,
    from: &Path,
    remote: &str,
    mode: u32,
    done: &mut u64,
    report: &mut (impl FnMut(u64) + Send),
) -> Result<()> {
    let never = AtomicBool::new(false);
    let mut buffer = vec![0u8; 66000];
    let mut sync = libadb::sync::open(wire, &mut buffer)
        .await
        .map_err(connection_error)?;
    sync.send_start(remote, mode)
        .await
        .map_err(connection_error)?;
    let mut file = tokio::fs::File::open(from)
        .await
        .map_err(connection_error)?;
    let mut chunk = vec![0u8; 65536];
    loop {
        let n = file.read(&mut chunk).await.map_err(connection_error)?;
        if n == 0 {
            break;
        }
        files::interruptible(sync.send_data(&chunk[..n]), &never, UPLOAD_STALL)
            .await
            .map_err(|_| PocError::new("timeout", "The transfer to the TV stalled."))?
            .map_err(connection_error)?;
        *done += n as u64;
        report(*done);
    }
    sync.send_done(0).await.map_err(connection_error)?;
    sync.quit().await.map_err(connection_error)?;
    Ok(())
}
/// Reads `Success: created install session [1234]`.
pub fn install_session(output: &str) -> Option<u32> {
    let start = output.find('[')? + 1;
    let end = start + output[start..].find(']')?;
    output.starts_with("Success").then(|| output[start..end].parse().ok())?
}
const SHELL_OUTPUT_LIMIT: usize = 1024 * 1024;
const OUTPUT_LIMIT: &str = "outputLimit";
const OUTPUT_LIMIT_MESSAGE: &str = "The TV returned more output than this command allows.";
/// By the time this is reported the command has finished and its stream is closed, so the
/// connection is as good as it was. Every other failure can leave it mid-conversation.
fn keeps_connection(error: &PocError) -> bool {
    error.kind == OUTPUT_LIMIT
}
const CUT_NOTE: &str = "\n\nBridgecoil stopped showing this at 1 MB. The command ran to the end.\n";
/// A busy 1080p screen is several megabytes as PNG, and base64 adds a third.
const LARGE_OUTPUT_LIMIT: usize = 24 * 1024 * 1024;
fn connection_error(e: impl std::fmt::Display) -> PocError {
    PocError::new("connection", e.to_string())
}
/// Google Play Protect ends the install this way when someone leaves its prompt on the TV, which
/// is the owner's call to make rather than something to switch off behind their back.
pub fn install_rejection(output: &str) -> String {
    if output.contains("INSTALL_FAILED_VERIFICATION_FAILURE") {
        return "Google Play Protect stopped this install on the TV. Try again, and on the TV \
                choose More details, then Install anyway."
            .into();
    }
    format!("Android rejected the installation: {}", output.trim())
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
    command_output_limited(wire, cmd, SHELL_OUTPUT_LIMIT, Overflow::Refuse).await
}
/// What to do when a command says more than its limit. A command Bridgecoil runs for itself can't
/// be trusted past the limit, but someone reading a log still wants the part that fits.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Overflow {
    Refuse,
    Cut,
}
async fn command_output_limited(
    wire: &mut Wire,
    cmd: &str,
    limit: usize,
    overflow: Overflow,
) -> Result<crate::workspace::driver::ShellOutput> {
    use libadb::shell::v2::{self, Frame};
    let mut rx = vec![0u8; 65536];
    let mut shell = v2::open(wire, cmd, &mut rx)
        .await
        .map_err(connection_error)?;
    shell.close_stdin().await.map_err(connection_error)?;
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut over_limit = false;
    let exit = loop {
        let frame = shell.read_frame().await.map_err(connection_error)?;
        let (Frame::Stdout(b) | Frame::Stderr(b)) = &frame else {
            if let Frame::Exit(code) = frame {
                break code;
            }
            continue;
        };
        if stdout.len() + stderr.len() + b.len() > limit {
            // Keep reading to the end: the stream has to finish for the connection to stay usable.
            over_limit = true;
            continue;
        }
        if matches!(frame, Frame::Stdout(_)) {
            stdout.extend_from_slice(b);
        } else {
            stderr.extend_from_slice(b);
        }
    };
    shell.close().await.map_err(connection_error)?;
    if over_limit {
        if overflow == Overflow::Refuse {
            return Err(PocError::new(OUTPUT_LIMIT, OUTPUT_LIMIT_MESSAGE));
        }
        stderr.extend_from_slice(CUT_NOTE.as_bytes());
    }
    Ok(crate::workspace::driver::ShellOutput {
        exit_code: Some(exit),
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
    })
}
#[async_trait::async_trait]
impl crate::workspace::driver::ShellDriver for Client {
    fn target_label(&self) -> Option<String> {
        self.status().map(|info| info.model)
    }
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
    async fn shell_for_person(
        &mut self,
        target: &crate::workspace::types::Target,
        command: &str,
    ) -> Result<crate::workspace::driver::ShellOutput> {
        self.execute_limited(target, command, SHELL_OUTPUT_LIMIT, Overflow::Cut)
            .await
    }
    async fn shell(
        &mut self,
        target: &crate::workspace::types::Target,
        command: &str,
    ) -> Result<crate::workspace::driver::ShellOutput> {
        self.execute_target(target, command).await
    }
    async fn shell_large(
        &mut self,
        target: &crate::workspace::types::Target,
        command: &str,
    ) -> Result<crate::workspace::driver::ShellOutput> {
        self.execute_limited(target, command, LARGE_OUTPUT_LIMIT, Overflow::Refuse)
            .await
    }
}

#[cfg(test)]
mod install_session_tests {
    use super::install_session;
    #[test]
    fn reads_the_number_android_gives_a_session() {
        assert_eq!(install_session("Success: created install session [1394583712]"), Some(1394583712));
        assert_eq!(install_session("Failure [INSTALL_FAILED_INTERNAL_ERROR]"), None);
        assert_eq!(install_session("Success: created install session"), None);
        assert_eq!(install_session("Success: [x]"), None);
    }
}
#[cfg(test)]
mod output_limit_tests {
    use super::*;

    #[test]
    fn only_a_finished_command_leaves_the_connection_usable() {
        assert!(keeps_connection(&PocError::new(
            OUTPUT_LIMIT,
            OUTPUT_LIMIT_MESSAGE
        )));
        for kind in ["connection", "timeout", "staleTarget", "input"] {
            assert!(!keeps_connection(&PocError::new(kind, "x")), "{kind}");
        }
    }
}
