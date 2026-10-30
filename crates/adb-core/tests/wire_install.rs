//! Independent minimal adbd wire peer: exercises the production adapter, not a mocked Client.
use adb_core::{apk, backend::Client};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
};
fn packet(s: &mut TcpStream, cmd: &[u8; 4], a: u32, b: u32, data: &[u8]) -> std::io::Result<()> {
    let c = u32::from_le_bytes(*cmd);
    for n in [c, a, b, data.len() as u32, 0, c ^ u32::MAX] {
        s.write_all(&n.to_le_bytes())?;
    }
    s.write_all(data)?;
    Ok(())
}
fn send(s: &mut TcpStream, cmd: &[u8; 4], a: u32, b: u32, data: &[u8]) {
    packet(s, cmd, a, b, data).unwrap();
}
fn read(s: &mut TcpStream) -> Option<([u8; 4], u32, u32, Vec<u8>)> {
    let mut h = [0; 24];
    s.read_exact(&mut h).ok()?;
    let n = |i| u32::from_le_bytes(h[i..i + 4].try_into().unwrap());
    let mut d = vec![0; n(12) as usize];
    s.read_exact(&mut d).ok()?;
    Some((h[..4].try_into().unwrap(), n(4), n(8), d))
}
fn frame(id: u8, data: &[u8]) -> Vec<u8> {
    let mut r = vec![id];
    r.extend_from_slice(&(data.len() as u32).to_le_bytes());
    r.extend_from_slice(data);
    r
}
#[derive(Default)]
struct Seen {
    commands: Vec<String>,
    uploaded: Vec<u8>,
}
fn peer(listener: TcpListener, seen: Arc<Mutex<Seen>>, interrupt: bool) {
    let (mut s, _) = listener.accept().unwrap();
    s.set_read_timeout(Some(std::time::Duration::from_secs(15)))
        .unwrap();
    let (c, _, _, _) = read(&mut s).unwrap();
    assert_eq!(&c, b"CNXN");
    send(
        &mut s,
        b"CNXN",
        0x01000001,
        65536,
        b"device::features=shell_v2,stat_v2,sendrecv_v2\0",
    );
    let mut local = 0;
    let remote = 1;
    let mut dest = String::new();
    let mut pending_exit = false;
    let mut sync_data = Vec::new();
    while let Some((cmd, a, _, data)) = read(&mut s) {
        match &cmd {
            b"OPEN" => {
                local = a;
                dest = String::from_utf8_lossy(&data)
                    .trim_end_matches('\0')
                    .to_string();
                sync_data.clear();
                send(&mut s, b"OKAY", remote, local, &[]);
                if dest.starts_with("shell,") {
                    seen.lock().unwrap().commands.push(dest.clone());
                }
            }
            b"WRTE" => {
                send(&mut s, b"OKAY", remote, local, &[]);
                if dest == "sync:" {
                    sync_data.extend_from_slice(&data);
                    loop {
                        if sync_data.len() < 8 {
                            break;
                        }
                        let id: [u8; 4] = sync_data[..4].try_into().unwrap();
                        let len = u32::from_le_bytes(sync_data[4..8].try_into().unwrap()) as usize;
                        let needed = if &id == b"DONE" { 8 } else { 8 + len };
                        if sync_data.len() < needed {
                            break;
                        }
                        if &id == b"DATA" {
                            seen.lock()
                                .unwrap()
                                .uploaded
                                .extend_from_slice(&sync_data[8..needed]);
                        }
                        if &id == b"DONE" {
                            send(&mut s, b"WRTE", remote, local, b"OKAY\0\0\0\0");
                        }
                        sync_data.drain(..needed);
                    }
                } else if data.first() == Some(&4) {
                    if interrupt && dest.contains("pm install ") {
                        return;
                    }
                    let output = if dest.contains("getprop") {
                        "Fixture TV\n14\n34\narm64-v8a\n"
                    } else if dest.contains("pm install ") {
                        "Success\n"
                    } else if dest.contains("dumpsys package ") {
                        "Packages:\n  versionCode=7 minSdk=23 targetSdk=35\n"
                    } else {
                        ""
                    };
                    send(&mut s, b"WRTE", remote, local, &frame(1, output.as_bytes()));
                    pending_exit = true;
                }
            }
            b"OKAY" => {
                if pending_exit {
                    pending_exit = false;
                    send(&mut s, b"WRTE", remote, local, &frame(3, &[0]));
                }
            }
            b"CLSE" => {
                if let Err(e) = packet(&mut s, b"CLSE", remote, a, &[]) {
                    match e.kind() {
                        std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::ConnectionReset => {
                            return;
                        }
                        _ => panic!("close acknowledgement: {e}"),
                    }
                }
            }
            _ => {}
        }
    }
}
async fn exercise(interrupt: bool) {
    let dir = tempfile::tempdir().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = listener.local_addr().unwrap().to_string();
    let seen = Arc::new(Mutex::new(Seen::default()));
    let other = seen.clone();
    let handle = thread::spawn(move || peer(listener, other, interrupt));
    let mut client = Client::new(dir.path().into());
    let info = client.connect(endpoint, true).await.unwrap();
    assert_eq!(info.model, "Fixture TV");
    assert!(!info.encrypted);
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/metadata.apk");
    let apk = apk::inspect(&path).unwrap();
    let mut stages = Vec::new();
    let result = client
        .install(&info.id, &path, &apk, |p| stages.push(p.stage))
        .await;
    if interrupt {
        assert_eq!(result.unwrap_err().kind, "unknownInstall");
        assert!(client.status().is_none());
    } else {
        result.unwrap();
        assert!(stages.contains(&"verifying".into()));
        assert!(client.status().is_some());
    }
    client.disconnect();
    handle.join().unwrap();
    let seen = seen.lock().unwrap();
    assert_eq!(seen.uploaded, std::fs::read(path).unwrap());
    assert_eq!(
        seen.commands
            .iter()
            .filter(|s| s.contains("pm install "))
            .count(),
        1
    );
    assert!(
        seen.commands
            .iter()
            .any(|s| s.contains("pm install -r --user 0 '/data/local/tmp/adb-studio-"))
    );
}
#[tokio::test]
async fn direct_wire_upload_install_and_readback() {
    exercise(false).await;
}
#[tokio::test]
async fn interrupted_installer_is_unknown_and_never_retried() {
    exercise(true).await;
}
