use adb_core::{apk, identity};
#[test]
fn identity_roundtrip_preserves_public_key() {
    let dir = tempfile::tempdir().unwrap();
    let a = identity::load_or_create(dir.path()).unwrap();
    let b = identity::load_or_create(dir.path()).unwrap();
    assert_eq!(a.public_key_wire(), b.public_key_wire());
}
#[cfg(unix)]
#[test]
fn identity_is_private() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    identity::load_or_create(dir.path()).unwrap();
    assert_eq!(
        std::fs::metadata(dir.path().join("host.pk8"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
}
#[test]
fn package_id_is_not_a_shell_program() {
    assert!(apk::valid_package("org.example.player"));
    for v in ["", "foo", "a.;id", "a.b'", "a.b\n", "a..b"] {
        assert!(!apk::valid_package(v), "{v}");
    }
}
#[test]
fn corrupt_apk_is_rejected() {
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), b"not an apk").unwrap();
    assert!(apk::inspect(file.path()).is_err());
}

#[test]
fn reads_actual_binary_manifest() {
    let p =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/metadata.apk");
    let info = apk::inspect(&p).unwrap();
    assert_eq!(info.package, "nl.neerdael.adbstudio.fixture");
    assert_eq!(info.version_code, 7);
    assert_eq!(info.version_name.as_deref(), Some("0.0.7"));
    assert_eq!(info.min_sdk, Some(23));
    assert!(info.abis.is_empty());
    assert_eq!(info.sha256.len(), 64);
}
