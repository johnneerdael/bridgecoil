use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{fs::File, io::Read, path::Path};

pub const MAX_APK_BYTES: u64 = 1024 * 1024 * 1024;
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApkInfo {
    pub package: String,
    pub version_code: u64,
    pub version_name: Option<String>,
    pub min_sdk: Option<u32>,
    pub abis: Vec<String>,
    pub bytes: u64,
    pub sha256: String,
}
pub fn valid_package(value: &str) -> bool {
    value.contains('.')
        && value.split('.').all(|p| {
            !p.is_empty()
                && p.bytes().enumerate().all(|(i, c)| {
                    c.is_ascii_alphabetic() || c == b'_' || i > 0 && c.is_ascii_digit()
                })
        })
}
fn number(value: &str) -> Option<u64> {
    value
        .strip_prefix("0x")
        .and_then(|v| u64::from_str_radix(v, 16).ok())
        .or_else(|| value.parse().ok())
}
pub fn inspect(path: &Path) -> Result<ApkInfo, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let bytes = file.metadata().map_err(|e| e.to_string())?.len();
    if bytes == 0 || bytes > MAX_APK_BYTES {
        return Err("Choose a nonempty APK smaller than 1 GiB.".into());
    }
    let mut hash = Sha256::new();
    let mut chunk = [0; 65536];
    loop {
        let n = file.read(&mut chunk).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&chunk[..n]);
    }
    let mut zip = zip::ZipArchive::new(File::open(path).map_err(|e| e.to_string())?)
        .map_err(|_| "The selected file is not a readable APK ZIP archive.")?;
    if zip.len() > 100_000 {
        return Err("APK has too many entries.".into());
    }
    let mut manifest = zip
        .by_name("AndroidManifest.xml")
        .map_err(|_| "APK has no AndroidManifest.xml.")?;
    if manifest.size() > 2 * 1024 * 1024 {
        return Err("APK manifest is too large.".into());
    }
    let mut data = Vec::new();
    manifest
        .by_ref()
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut data)
        .map_err(|e| e.to_string())?;
    if data.len() > 2 * 1024 * 1024 {
        return Err("APK manifest exceeds the limit.".into());
    }
    let doc = std::panic::catch_unwind(|| axmldecoder::parse(&data))
        .map_err(|_| "Malformed Android manifest.")?
        .map_err(|_| "Could not decode the APK manifest.")?;
    let Some(axmldecoder::Node::Element(root)) = doc.get_root() else {
        return Err("APK manifest has no root element.".into());
    };
    if root.get_tag() != "manifest" {
        return Err("Invalid APK manifest root.".into());
    }
    let attrs = root.get_attributes();
    let package = attrs
        .get("package")
        .filter(|v| valid_package(v))
        .ok_or("APK has an invalid package identifier.")?
        .clone();
    if attrs.contains_key("split") {
        return Err(
            "This POC installs standalone APKs. Choose the complete APK rather than a split."
                .into(),
        );
    }
    let low = attrs
        .get("android:versionCode")
        .and_then(|v| number(v))
        .ok_or("APK version code is missing or unreadable.")?;
    let high = attrs
        .get("android:versionCodeMajor")
        .and_then(|v| number(v))
        .unwrap_or(0);
    let min_sdk = root.get_children().iter().find_map(|n| match n {
        axmldecoder::Node::Element(e) if e.get_tag() == "uses-sdk" => e
            .get_attributes()
            .get("android:minSdkVersion")
            .and_then(|v| number(v))
            .and_then(|v| u32::try_from(v).ok()),
        _ => None,
    });
    let version_name = attrs.get("android:versionName").cloned();
    drop(manifest);
    let mut abis: Vec<_> = zip
        .file_names()
        .filter_map(|name| {
            let mut parts = name.split('/');
            (parts.next()? == "lib")
                .then(|| parts.next().map(str::to_owned))
                .flatten()
        })
        .collect();
    abis.sort();
    abis.dedup();
    Ok(ApkInfo {
        package,
        version_code: (high << 32) | low,
        version_name,
        min_sdk,
        abis,
        bytes,
        sha256: hex::encode(hash.finalize()),
    })
}
