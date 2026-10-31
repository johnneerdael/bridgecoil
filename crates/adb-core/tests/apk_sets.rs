use adb_core::{
    apk,
    apk_set::{assemble_set, inspect_part},
};
use std::path::PathBuf;
fn path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/apks")
        .join(name)
}
#[test]
fn accepts_a_base_and_matching_config_split() {
    let base = inspect_part(&path("v2-rsa.apk")).unwrap();
    let split = inspect_part(&path("config-test.apk")).unwrap();
    assert_eq!(split.info.split.as_deref(), Some("config.test"));
    let plan = assemble_set(vec![split, base], 34, &["arm64-v8a".into()]).unwrap();
    assert!(plan.files[0].info.split.is_none());
    assert_eq!(plan.files.len(), 2);
    assert_eq!(plan.package, "org.bridgecoil.fixture.install");
}
#[test]
fn rejects_missing_base_duplicate_split_and_mixed_versions() {
    let base = inspect_part(&path("v2-rsa.apk")).unwrap();
    let split = inspect_part(&path("config-test.apk")).unwrap();
    assert!(assemble_set(vec![split.clone()], 34, &[]).is_err());
    assert!(assemble_set(vec![base.clone(), split.clone(), split], 34, &[]).is_err());
    let update = inspect_part(&path("update-v2.apk")).unwrap();
    assert!(
        assemble_set(
            vec![update, inspect_part(&path("config-test.apk")).unwrap()],
            34,
            &[]
        )
        .is_err()
    );
}
#[test]
fn validates_required_parts_signers_and_device_compatibility() {
    let base = inspect_part(&path("v2-rsa.apk")).unwrap();
    let mut split = inspect_part(&path("config-test.apk")).unwrap();
    split.info.required_splits = vec!["missing.feature".into()];
    assert!(assemble_set(vec![base.clone(), split], 34, &[]).is_err());
    let mut wrong = inspect_part(&path("config-test.apk")).unwrap();
    wrong.signing = inspect_part(&path("other-signer.apk")).unwrap().signing;
    assert!(assemble_set(vec![base.clone(), wrong], 34, &[]).is_err());
    assert!(assemble_set(vec![base.clone()], 23, &[]).is_err());
    let mut requires = base;
    requires.info.requires_splits = true;
    assert!(assemble_set(vec![requires], 34, &[]).is_err());
}
#[test]
fn malformed_and_unsigned_inputs_are_not_installable() {
    assert!(inspect_part(&path("unsigned.apk")).is_err());
    assert!(inspect_part(&path("tampered.apk")).is_err());
    assert!(apk::inspect(&path("config-test.apk")).is_err());
}
