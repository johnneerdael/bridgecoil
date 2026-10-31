pub mod fetch;
pub mod groups;
pub mod store;

use crate::backend::PocError;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;

pub const MAX_LIST_BYTES: usize = 8 * 1024 * 1024;
pub const MIN_RECOGNISED: usize = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Level {
    Recommended,
    Advanced,
    Expert,
    Unsafe,
}
impl Level {
    fn parse(value: &str) -> Option<Self> {
        [
            ("recommended", Self::Recommended),
            ("advanced", Self::Advanced),
            ("expert", Self::Expert),
            ("unsafe", Self::Unsafe),
        ]
        .into_iter()
        .find_map(|(name, level)| value.eq_ignore_ascii_case(name).then_some(level))
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rating {
    pub level: Option<Level>,
    pub source: Option<String>,
    pub description: String,
    pub dependencies: Vec<String>,
    pub needed_by: Vec<String>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RatingList {
    entries: HashMap<String, Rating>,
}
impl RatingList {
    pub fn get(&self, package: &str) -> Option<&Rating> {
        self.entries.get(package)
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn recognised(&self) -> usize {
        self.entries.values().filter(|r| r.level.is_some()).count()
    }
    fn has_unsafe(&self) -> bool {
        self.entries
            .values()
            .any(|r| r.level == Some(Level::Unsafe))
    }
}

fn text(entry: &Map<String, Value>, key: &str) -> Option<String> {
    entry.get(key)?.as_str().map(str::to_owned)
}
fn names(entry: &Map<String, Value>, key: &str) -> Vec<String> {
    entry
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}
/// Read by hand, because a typed struct would refuse a whole file over one odd field.
pub fn parse_list(raw: &str) -> Result<RatingList, PocError> {
    let invalid = || PocError::new("ratingsInvalid", "The ratings list could not be read.");
    let value: Value = serde_json::from_str(raw).map_err(|_| invalid())?;
    let packages = value.as_object().ok_or_else(invalid)?;
    let entries = packages
        .iter()
        .filter_map(|(package, entry)| {
            let entry = entry.as_object()?;
            Some((
                package.clone(),
                Rating {
                    level: text(entry, "removal").and_then(|v| Level::parse(&v)),
                    source: text(entry, "list"),
                    description: text(entry, "description").unwrap_or_default(),
                    dependencies: names(entry, "dependencies"),
                    needed_by: names(entry, "neededBy"),
                },
            ))
        })
        .collect();
    Ok(RatingList { entries })
}

/// Stops a renamed field or bad commit from quietly removing every rating and the Unsafe block.
pub fn check_acceptable(new: &RatingList, stored: Option<&RatingList>) -> Result<(), PocError> {
    let recognised = new.recognised();
    let shrank = stored.is_some_and(|s| recognised * 2 < s.recognised());
    if recognised < MIN_RECOGNISED || !new.has_unsafe() || shrank {
        return Err(PocError::new(
            "ratingsRejected",
            "This ratings update looks wrong, so the current list was kept.",
        ));
    }
    Ok(())
}
