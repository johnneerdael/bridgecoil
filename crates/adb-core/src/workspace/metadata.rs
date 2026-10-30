use super::commands::{component_arg, package_arg};
use crate::backend::PocError;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Permission {
    pub name: String,
    pub granted: bool,
    pub protection: Option<u8>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Metadata {
    pub package: String,
    pub label: Option<String>,
    pub icon: Option<String>,
    pub version_name: Option<String>,
    pub version_code: Option<u64>,
    pub system: Option<bool>,
    pub enabled: Option<bool>,
    pub installed: Option<bool>,
    pub target_sdk: Option<u32>,
    pub activities: Vec<String>,
    pub permissions: Vec<Permission>,
    pub signers: Vec<String>,
    pub error: Option<String>,
}
pub fn parse_metadata(raw: &str, requested: &[String]) -> Result<Vec<Metadata>, PocError> {
    if raw.len() > 1_048_576 || requested.len() > 4096 {
        return Err(PocError::new(
            "metadata",
            "Metadata exceeds inspection limits.",
        ));
    }
    let line = raw
        .lines()
        .find_map(|l| l.strip_prefix("__BRIDGECOIL_JSON__"))
        .ok_or_else(|| {
            PocError::new("metadata", "The TV did not return structured app metadata.")
        })?;
    let value: serde_json::Value = serde_json::from_str(line)
        .map_err(|_| PocError::new("metadata", "Malformed app metadata response."))?;
    if let Some(e) = value["error"].as_str() {
        return Err(PocError::new(
            "metadata",
            e.chars().take(256).collect::<String>(),
        ));
    }
    let rows = value["items"]
        .as_array()
        .ok_or_else(|| PocError::new("metadata", "Missing app metadata entries."))?;
    let mut results = Vec::new();
    for row in rows.iter().take(1024) {
        let Ok(mut item) = serde_json::from_value::<Metadata>(row.clone()) else {
            continue;
        };
        if package_arg(&item.package).is_err()
            || !requested.contains(&item.package)
            || results.iter().any(|m: &Metadata| m.package == item.package)
        {
            continue;
        }
        item.label = item
            .label
            .map(|s| s.chars().filter(|c| !c.is_control()).take(256).collect());
        item.version_name = item.version_name.map(|s| s.chars().take(256).collect());
        item.error = item.error.map(|s| s.chars().take(256).collect());
        item.activities
            .retain(|c| component_arg(c).is_ok() && c.split('/').next() == Some(&item.package));
        item.activities.truncate(64);
        item.permissions.retain(|p| {
            p.name.len() <= 256
                && p.name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "._".contains(c))
        });
        item.permissions.truncate(256);
        item.signers
            .retain(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()));
        item.signers.truncate(16);
        item.icon = item.icon.filter(|icon| {
            if icon.len() > 43700 {
                return false;
            }
            let Ok(bytes) = STANDARD.decode(icon) else {
                return false;
            };
            bytes.len() >= 24
                && bytes.len() <= 32768
                && bytes.starts_with(b"\x89PNG\r\n\x1a\n")
                && bytes[12..16] == *b"IHDR"
                && u32::from_be_bytes(bytes[16..20].try_into().unwrap()) == 64
                && u32::from_be_bytes(bytes[20..24].try_into().unwrap()) == 64
        });
        results.push(item);
    }
    Ok(results)
}
