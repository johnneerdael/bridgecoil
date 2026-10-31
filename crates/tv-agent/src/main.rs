//! Runs in Termux on the TV. Gets shell access by connecting to the TV's own ADB, so that nothing
//! else has to be installed and nothing depends on a phone.
mod mdns;

use libadb::{
    Connection, Feature, TokioTcp,
    keys::{AdbKey, rsa::rand_core::OsRng, zeroize::Zeroizing},
    shell::v2::{self, Frame},
    tls::{TlsClientConfig, TlsIdentity},
    transport::tls::MaybeTls,
};
use std::{
    env, fs,
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::PathBuf,
    process::ExitCode,
    time::Duration,
};
use tokio::{net::TcpStream, time::timeout};

const CLASSIC_PORT: u16 = 5555;

fn key_path() -> Result<PathBuf, String> {
    let home = env::var("HOME").map_err(|_| "HOME isn't set".to_owned())?;
    Ok(PathBuf::from(home).join(".bridgecoil/agent.pk8"))
}

fn identity() -> Result<(AdbKey, TlsClientConfig), String> {
    let path = key_path()?;
    let key = match fs::read(&path) {
        Ok(bytes) => AdbKey::from_pkcs8_der(&Zeroizing::new(bytes), &mut OsRng, "Bridgecoil TV")
            .map_err(|e| e.to_string())?,
        Err(_) => {
            let key = AdbKey::generate(&mut OsRng, "Bridgecoil TV").map_err(|e| e.to_string())?;
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let der = key.to_pkcs8_der().map_err(|e| e.to_string())?;
            fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&path)
                .and_then(|mut file| file.write_all(der.as_bytes()))
                .map_err(|e| e.to_string())?;
            key
        }
    };
    let certificate = TlsIdentity::from_key(&key, &mut OsRng).map_err(|e| e.to_string())?;
    let tls = TlsClientConfig::adb(&certificate).map_err(|e| e.to_string())?;
    Ok((key, tls))
}

/// Pairs with the TV's pairing screen, which has to be open so that it offers a port.
async fn pair(code: &str) -> Result<(), String> {
    if code.len() != 6 || !code.bytes().all(|b| b.is_ascii_digit()) {
        return Err("A pairing code is six digits".to_owned());
    }
    let port = mdns::find(mdns::PAIRING_SERVICE, Duration::from_secs(10))
        .await
        .ok_or("The TV's pairing screen isn't open")?;
    let (key, tls) = identity()?;
    let socket = TcpStream::connect(("127.0.0.1", port))
        .await
        .map_err(|e| e.to_string())?;
    let mut wire = MaybeTls::plain(TokioTcp::new(socket));
    let mut random = OsRng;
    let pairing = libadb::pairing::pair(&mut wire, &tls, &key, code, &mut random);
    let result = timeout(Duration::from_secs(30), pairing)
        .await
        .map_err(|_| "Pairing timed out".to_owned())?;
    result.map(|_| ()).map_err(|e| e.to_string())
}

/// Runs `command` as the shell user, and returns its exit code.
async fn run(port: u16, command: &str) -> Result<u8, String> {
    let (key, tls) = identity()?;
    let socket = TcpStream::connect(("127.0.0.1", port))
        .await
        .map_err(|e| e.to_string())?;
    socket.set_nodelay(true).map_err(|e| e.to_string())?;
    let handshake = Connection::connect_tls(
        MaybeTls::plain(TokioTcp::new(socket)),
        key,
        &[Feature::ShellV2],
        &tls,
    );
    let mut wire: Connection<MaybeTls<TokioTcp>> = timeout(Duration::from_secs(25), handshake)
        .await
        .map_err(|_| "Connecting timed out".to_owned())?
        .map_err(|e| e.to_string())?;
    let mut buffer = vec![0; 65536];
    let mut shell = v2::open(&mut wire, command, &mut buffer)
        .await
        .map_err(|e| e.to_string())?;
    shell.close_stdin().await.map_err(|e| e.to_string())?;
    let mut out = std::io::stdout();
    loop {
        match shell.read_frame().await.map_err(|e| e.to_string())? {
            Frame::Stdout(bytes) | Frame::Stderr(bytes) => {
                out.write_all(&bytes).map_err(|e| e.to_string())?;
            }
            Frame::Exit(code) => return Ok(code),
            _ => {}
        }
    }
}

/// The port to connect to: wireless debugging if the TV has it, or classic network ADB.
async fn find_port(wait: Duration) -> Option<u16> {
    if let Some(port) = mdns::find(mdns::CONNECT_SERVICE, wait).await {
        return Some(port);
    }
    let classic = TcpStream::connect(("127.0.0.1", CLASSIC_PORT));
    matches!(timeout(Duration::from_millis(500), classic).await, Ok(Ok(_))).then_some(CLASSIC_PORT)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args.as_slice() {
        ["find"] => match find_port(Duration::from_secs(8)).await {
            Some(port) => {
                println!("{port}");
                Ok(0)
            }
            None => Err("The TV isn't offering ADB yet".to_owned()),
        },
        ["pair", code] => pair(code).await.map(|()| 0),
        ["run", command] => match find_port(Duration::from_secs(8)).await {
            Some(port) => run(port, command).await,
            None => Err("The TV isn't offering ADB yet".to_owned()),
        },
        _ => Err("Usage: bridgecoil-agent find | pair <code> | run <command>".to_owned()),
    };
    match result {
        Ok(0) => ExitCode::SUCCESS,
        Ok(code) => ExitCode::from(code),
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}
