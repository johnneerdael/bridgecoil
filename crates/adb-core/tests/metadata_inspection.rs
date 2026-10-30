use adb_core::workspace::metadata::parse_metadata;
#[test]
fn bundled_inspector_matches_its_audited_source_and_build_manifest() {
    use sha2::{Digest, Sha256};
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../assets/inspector-build.json")).unwrap();
    assert_eq!(
        manifest["sourceSha256"],
        hex::encode(Sha256::digest(include_bytes!(
            "../../../assets/inspector/BridgecoilInspector.java"
        )))
    );
    assert_eq!(
        manifest["jarSha256"],
        hex::encode(Sha256::digest(include_bytes!(
            "../assets/bridgecoil-inspector.jar"
        )))
    );
}
#[test]
fn accepts_only_requested_packages_and_valid_components() {
    let raw = "VM warning\n__BRIDGECOIL_JSON__{\"items\":[{\"package\":\"org.demo\",\"label\":\"Player <TV>\",\"activities\":[\"org.demo/.Main\",\"org.demo/.Main;reboot\"],\"signers\":[\"bad\"]},{\"package\":\"org.other\",\"label\":\"Other\"}]}\n";
    let rows = parse_metadata(raw, &["org.demo".into()]).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].label.as_deref(), Some("Player <TV>"));
    assert_eq!(rows[0].activities, vec!["org.demo/.Main"]);
    assert!(rows[0].signers.is_empty());
}
#[test]
fn rejects_unbounded_or_non_png_icons_and_reports_helper_failure() {
    let raw = "__BRIDGECOIL_JSON__{\"items\":[{\"package\":\"org.demo\",\"icon\":\"PHN2Zz4=\"}]}";
    let rows = parse_metadata(raw, &["org.demo".into()]).unwrap();
    assert!(rows[0].icon.is_none());
    assert!(parse_metadata("__BRIDGECOIL_JSON__{\"error\":\"Permission denied\"}", &[]).is_err());
    assert!(parse_metadata(&"x".repeat(1048577), &[]).is_err());
}
