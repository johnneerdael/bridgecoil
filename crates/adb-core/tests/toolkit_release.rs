use adb_core::{
    backend::PocError,
    toolkit::{
        catalog::{ToolId, select_asset, spec},
        release::{Asset, Release, ReleaseSource, fetch_tool, parse_release},
    },
};
use sha2::{Digest, Sha256};
use std::{cell::Cell, fs, path::Path};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/fixtures/apks/v2-rsa.apk"
);

fn asset(name: &str) -> Asset {
    Asset {
        name: name.into(),
        url: format!("https://example.test/{name}"),
        size: 10,
        digest: None,
    }
}
fn names(values: &[&str]) -> Vec<Asset> {
    values.iter().map(|n| asset(n)).collect()
}
fn abis(values: &[&str]) -> Vec<String> {
    values.iter().map(|v| (*v).to_owned()).collect()
}

#[test]
fn shizuku_next_never_picks_the_debug_build() {
    let assets = names(&[
        "shizuku-v14.0.31-next-debug.apk",
        "shizuku-v14.0.31-next.apk",
    ]);
    let picked = select_asset(ToolId::ShizukuNext, &assets, &abis(&["arm64-v8a"])).unwrap();
    assert_eq!(picked.name, "shizuku-v14.0.31-next.apk");
}

#[test]
fn porter_and_its_companion_are_told_apart() {
    let assets = names(&[
        "build-info.json",
        "porter-compat-v0.8.1-rc0-release.apk",
        "porter-v0.8.1-rc0-release.apk",
        "SHA256SUMS",
    ]);
    let any = abis(&["armeabi-v7a"]);
    assert_eq!(
        select_asset(ToolId::Porter, &assets, &any).unwrap().name,
        "porter-v0.8.1-rc0-release.apk"
    );
    assert_eq!(
        select_asset(ToolId::PorterCompat, &assets, &any)
            .unwrap()
            .name,
        "porter-compat-v0.8.1-rc0-release.apk"
    );
}

#[test]
fn termux_follows_the_tv_abi_order() {
    let assets = names(&[
        "termux-app_v0.118.3+github-debug_arm64-v8a.apk",
        "termux-app_v0.118.3+github-debug_armeabi-v7a.apk",
        "termux-app_v0.118.3+github-debug_universal.apk",
        "termux-app_v0.118.3+github-debug_sha256sums",
    ]);
    let picked = select_asset(ToolId::Termux, &assets, &abis(&["armeabi-v7a", "armeabi"])).unwrap();
    assert_eq!(
        picked.name,
        "termux-app_v0.118.3+github-debug_armeabi-v7a.apk"
    );
}

#[test]
fn termux_falls_back_to_universal_for_an_unknown_abi() {
    let assets = names(&["termux-app_v0.118.3+github-debug_universal.apk"]);
    let picked = select_asset(ToolId::Termux, &assets, &abis(&["riscv64"])).unwrap();
    assert!(picked.name.ends_with("universal.apk"));
}

#[test]
fn a_missing_asset_is_a_clear_error() {
    let e = select_asset(
        ToolId::Porter,
        &names(&["notes.txt"]),
        &abis(&["arm64-v8a"]),
    )
    .unwrap_err();
    assert_eq!(e.kind, "toolUnavailable");
}

#[test]
fn every_tool_has_a_pinned_identity() {
    for id in [
        ToolId::ShizukuNext,
        ToolId::Porter,
        ToolId::PorterCompat,
        ToolId::Termux,
        ToolId::TermuxBoot,
    ] {
        let tool = spec(id);
        assert_eq!(tool.signer_sha256.len(), 64, "{}", tool.name);
        assert!(!tool.package.is_empty());
        assert!(tool.repo.contains(char::from(47)));
    }
    assert_eq!(
        spec(ToolId::Porter).signer_sha256,
        spec(ToolId::PorterCompat).signer_sha256
    );
    assert_eq!(
        spec(ToolId::Termux).signer_sha256,
        spec(ToolId::TermuxBoot).signer_sha256
    );
}

#[test]
fn reads_a_github_release() {
    let body = r#"{"tag_name":"v1","assets":[
        {"name":"a.apk","browser_download_url":"https://github.com/x/a.apk","size":12,"digest":"sha256:ab"},
        {"name":"b.apk","browser_download_url":"https://github.com/x/b.apk","size":3,"digest":null}]}"#;
    let release = parse_release(body).unwrap();
    assert_eq!(release.tag, "v1");
    assert_eq!(release.assets[0].digest.as_deref(), Some("ab"));
    assert_eq!(release.assets[1].digest, None);
}

#[test]
fn a_rate_limit_reply_is_not_a_release() {
    let e = parse_release(r#"{"message":"API rate limit exceeded"}"#).unwrap_err();
    assert_eq!(e.kind, "toolNetwork");
}

#[test]
fn only_https_github_downloads_are_accepted() {
    let body = r#"{"tag_name":"v1","assets":[
        {"name":"a.apk","browser_download_url":"http://evil.test/a.apk","size":1,"digest":null}]}"#;
    assert!(parse_release(body).unwrap().assets.is_empty());
}

struct Fake {
    bytes: Vec<u8>,
    digest: Option<String>,
    name: &'static str,
    downloads: Cell<u32>,
}
impl ReleaseSource for Fake {
    fn latest(&self, _: &str) -> Result<Release, PocError> {
        Ok(Release {
            tag: "v9".into(),
            assets: vec![Asset {
                name: self.name.into(),
                url: "https://github.com/x".into(),
                size: self.bytes.len() as u64,
                digest: self.digest.clone(),
            }],
        })
    }
    fn download(&self, _: &Asset, dest: &Path, _: u64) -> Result<(), PocError> {
        self.downloads.set(self.downloads.get() + 1);
        fs::write(dest, &self.bytes).map_err(|e| PocError::new("io", e.to_string()))
    }
}
fn fixture() -> Vec<u8> {
    fs::read(FIXTURE).unwrap()
}

#[test]
fn a_release_signed_by_anyone_else_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let source = Fake {
        bytes: fixture(),
        digest: None,
        name: "shizuku-v1-next.apk",
        downloads: Cell::new(0),
    };
    let e = fetch_tool(
        &source,
        ToolId::ShizukuNext,
        &abis(&["arm64-v8a"]),
        dir.path(),
    )
    .unwrap_err();
    assert_eq!(e.kind, "toolIdentity");
    assert!(
        fs::read_dir(dir.path()).unwrap().next().is_none(),
        "refused file removed"
    );
}

#[test]
fn a_wrong_digest_is_refused_before_inspection() {
    let dir = tempfile::tempdir().unwrap();
    let source = Fake {
        bytes: fixture(),
        digest: Some("00".repeat(32)),
        name: "shizuku-v1-next.apk",
        downloads: Cell::new(0),
    };
    let e = fetch_tool(
        &source,
        ToolId::ShizukuNext,
        &abis(&["arm64-v8a"]),
        dir.path(),
    )
    .unwrap_err();
    assert_eq!(e.kind, "toolIntegrity");
}

#[test]
fn a_correct_digest_still_needs_the_right_signer() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = fixture();
    let digest = hex::encode(Sha256::digest(&bytes));
    let source = Fake {
        bytes,
        digest: Some(digest),
        name: "porter-v1-release.apk",
        downloads: Cell::new(0),
    };
    let e = fetch_tool(&source, ToolId::Porter, &abis(&["arm64-v8a"]), dir.path()).unwrap_err();
    assert_eq!(e.kind, "toolIdentity");
}
