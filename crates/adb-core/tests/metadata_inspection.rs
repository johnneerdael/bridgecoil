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

#[test]
fn bundled_agents_match_their_build_manifest() {
    use sha2::{Digest, Sha256};
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../assets/agent/agent-build.json")).unwrap();
    for (abi, binary) in [
        (
            "armeabi-v7a",
            &include_bytes!("../assets/agent/armeabi-v7a/bridgecoil-agent")[..],
        ),
        (
            "arm64-v8a",
            &include_bytes!("../assets/agent/arm64-v8a/bridgecoil-agent")[..],
        ),
        (
            "x86_64",
            &include_bytes!("../assets/agent/x86_64/bridgecoil-agent")[..],
        ),
    ] {
        assert_eq!(manifest["binaries"][abi], hex::encode(Sha256::digest(binary)));
    }
}

/// Fails when the agent's source changes and `scripts/build-agent.py` wasn't run.
#[test]
fn bundled_agents_were_built_from_this_source() {
    use sha2::{Digest, Sha256};
    use std::{fs, path::Path};
    fn files(folder: &Path, found: &mut Vec<std::path::PathBuf>) {
        for entry in fs::read_dir(folder).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                files(&path, found);
            } else {
                found.push(path);
            }
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut found = Vec::new();
    files(&root.join("crates/tv-agent"), &mut found);
    found.sort();
    let mut hash = Sha256::new();
    for path in found {
        hash.update(fs::read(path).unwrap());
    }
    hash.update(without_app_version(&fs::read_to_string(root.join("Cargo.lock")).unwrap()));
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../assets/agent/agent-build.json")).unwrap();
    assert_eq!(manifest["sourceSha256"], hex::encode(hash.finalize()));
}

/// A release stamps its version into the app, and Cargo then rewrites that line of `Cargo.lock`.
/// The agent doesn't depend on the app, so the line says nothing about how the agent was built.
/// `scripts/build-agent.py` does the same.
fn without_app_version(lock: &str) -> String {
    let mut kept: Vec<&str> = Vec::new();
    for line in lock.split('\n') {
        if kept.last() == Some(&"name = \"adb-studio-app\"") && line.starts_with("version = ") {
            continue;
        }
        kept.push(line);
    }
    kept.join("\n")
}

#[test]
fn only_the_app_version_is_left_out_of_the_agent_source() {
    let lock = |app: &str, dependency: &str| {
        format!(
            "[[package]]\nname = \"adb-studio-app\"\nversion = \"{app}\"\n\n\
             [[package]]\nname = \"libadb\"\nversion = \"{dependency}\"\n"
        )
    };
    assert_eq!(
        without_app_version(&lock("0.1.7", "0.7.0")),
        without_app_version(&lock("0.1.10", "0.7.0"))
    );
    assert_ne!(
        without_app_version(&lock("0.1.7", "0.7.0")),
        without_app_version(&lock("0.1.7", "0.7.1"))
    );
}
