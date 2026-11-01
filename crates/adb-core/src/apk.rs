use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{fs::File, io::Read, path::Path};

pub const MAX_APK_BYTES: u64 = 1024 * 1024 * 1024;
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApkInfo {
    pub package: String,
    pub version_code: u64,
    pub version_name: Option<String>,
    pub min_sdk: Option<u32>,
    pub abis: Vec<String>,
    pub bytes: u64,
    pub sha256: String,
    pub split: Option<String>,
    pub required_splits: Vec<String>,
    pub config_for_split: Option<String>,
    pub requires_splits: bool,
    pub required_split_types: Vec<String>,
    pub split_types: Vec<String>,
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
pub fn valid_split_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
}
/// The everyday name for a processor type. Android's own name stays beside it in messages,
/// because that's what download pages list.
fn processor_name(abi: &str) -> &str {
    match abi {
        "arm64-v8a" => "64-bit ARM",
        "armeabi-v7a" | "armeabi" => "32-bit ARM",
        "x86_64" => "64-bit Intel",
        "x86" => "32-bit Intel",
        other => other,
    }
}
fn processors(abis: &[String]) -> String {
    let mut names: Vec<&str> = Vec::new();
    for abi in abis {
        let name = processor_name(abi);
        if !names.contains(&name) {
            names.push(name);
        }
    }
    format!("{} ({})", names.join(" and "), abis.join(", "))
}
/// A file with native code runs only if Android has one of its processor types. A 64-bit chip
/// often runs 32-bit Android, and only Android's side counts, so the message says which it is.
pub fn wrong_processor(file: &[String], tv: &[String]) -> Option<String> {
    let first = tv.first()?;
    if file.is_empty() || file.iter().any(|abi| tv.contains(abi)) {
        return None;
    }
    Some(format!(
        "This file is built for {}, but Android on this TV is {}. Look for the {first} version of this app.",
        processors(file),
        processors(tv),
    ))
}
fn android_name(sdk: u32) -> String {
    let name = match sdk {
        21 => "5.0",
        22 => "5.1",
        23 => "6",
        24 => "7.0",
        25 => "7.1",
        26 => "8.0",
        27 => "8.1",
        28 => "9",
        29 => "10",
        30 => "11",
        31 => "12",
        32 => "12L",
        33 => "13",
        34 => "14",
        35 => "15",
        36 => "16",
        other => return format!("Android API level {other}"),
    };
    format!("Android {name}")
}
/// Why Android would refuse this file on this TV, if it would. A TV that couldn't be read reports
/// 0 and no processors, which leaves the decision to Android.
pub fn unfit(info: &ApkInfo, sdk: u32, abis: &[String]) -> Option<String> {
    if let Some(needed) = info.min_sdk.filter(|min| sdk != 0 && *min > sdk) {
        return Some(format!(
            "This app needs {} or newer, and this TV has {}. Look for an older version of this app.",
            android_name(needed),
            android_name(sdk),
        ));
    }
    wrong_processor(&info.abis, abis)
}
pub fn inspect(path: &Path) -> Result<ApkInfo, String> {
    inspect_inner(path, false)
}
pub fn inspect_package(path: &Path) -> Result<ApkInfo, String> {
    inspect_inner(path, true)
}
fn inspect_inner(path: &Path, allow_split: bool) -> Result<ApkInfo, String> {
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
    let metadata = if allow_split {
        crate::manifest::parse_package(&data)
    } else {
        crate::manifest::parse(&data)
    }?;
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
        split: metadata.split,
        required_splits: metadata.required_splits,
        config_for_split: metadata.config_for_split,
        requires_splits: metadata.requires_splits,
        required_split_types: metadata.required_split_types,
        split_types: metadata.split_types,
        package: metadata.package,
        version_code: metadata.version_code,
        version_name: metadata.version_name,
        min_sdk: metadata.min_sdk,
        abis,
        bytes,
        sha256: hex::encode(hash.finalize()),
    })
}
