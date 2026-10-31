use adb_core::signing::{self, SignatureState};
use std::path::PathBuf;
fn path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/apks")
        .join(name)
}
#[test]
fn verifies_independent_sdk_rsa_ec_and_jar_fixtures() {
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(path("fixtures.json")).unwrap()).unwrap();
    for name in ["v2-rsa.apk", "v2-ec.apk", "v2-v3.apk", "v1-only.apk"] {
        let report = signing::inspect(&path(name)).unwrap();
        assert_eq!(
            report.state,
            SignatureState::Verified,
            "{name}: {:?}",
            report.reason
        );
        assert_eq!(
            report.signers[0].sha256,
            manifest["files"][name]["signerSha256"].as_str().unwrap()
        );
        for scheme in manifest["files"][name]["schemes"].as_array().unwrap() {
            assert!(
                report
                    .schemes
                    .iter()
                    .any(|s| Some(s.as_str()) == scheme.as_str())
            );
        }
    }
}
#[test]
fn rejects_changed_contents_and_distinguishes_unsigned() {
    assert_eq!(
        signing::inspect(&path("tampered.apk")).unwrap().state,
        SignatureState::Invalid
    );
    assert_eq!(
        signing::inspect(&path("unsigned.apk")).unwrap().state,
        SignatureState::Unsigned
    );
}
#[test]
fn signing_blocks_have_checked_lengths_and_bounded_record_counts() {
    assert!(signing::validate_block_limits(&[0; 32]).is_err());
    let mut value = Vec::new();
    let size = 24u64;
    value.extend(size.to_le_bytes());
    value.extend(size.to_le_bytes());
    value.extend(b"APK Sig Block 42");
    assert!(signing::validate_block_limits(&value).is_ok());
    value[0..8].copy_from_slice(&u64::MAX.to_le_bytes());
    assert!(signing::validate_block_limits(&value).is_err());
}
