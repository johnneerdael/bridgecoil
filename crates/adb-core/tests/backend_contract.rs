use adb_core::{
    apk::ApkInfo,
    backend::{Client, installed_version},
};
#[test]
fn installed_version_is_exact() {
    assert_eq!(
        installed_version("  versionCode=42 minSdk=23 targetSdk=35"),
        Some(42)
    );
    assert_eq!(
        installed_version("  versionCode=4294967297 minSdk=23"),
        Some(4294967297)
    );
    assert_eq!(installed_version("Unable to find package"), None);
}
#[tokio::test]
async fn invalid_pairing_code_never_opens_socket() {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    l.set_nonblocking(true).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let mut c = Client::new(dir.path().into());
    let e = c
        .pair(l.local_addr().unwrap().to_string(), "bad".into())
        .await
        .unwrap_err();
    assert_eq!(e.kind, "input");
    assert_eq!(
        l.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}
#[tokio::test]
async fn stale_install_is_rejected_before_reading_file() {
    let dir = tempfile::tempdir().unwrap();
    let mut c = Client::new(dir.path().into());
    let apk = ApkInfo {
        package: "org.example.test".into(),
        version_code: 1,
        version_name: None,
        min_sdk: None,
        abis: vec![],
        bytes: 12,
        sha256: "test".into(),
    };
    let e = c
        .install("expired", std::path::Path::new("missing.apk"), &apk, |_| {})
        .await
        .unwrap_err();
    assert_eq!(e.kind, "staleSession");
}
