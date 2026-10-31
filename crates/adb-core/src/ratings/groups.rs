use super::{Level, Rating};
use serde::Serialize;

/// Declared in list order: an app's position should reflect how readily it can go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Group {
    Installed,
    Recommended,
    Advanced,
    NotRated,
    Expert,
    Unsafe,
}

pub fn group_for(rating: Option<&Rating>, user_installed: bool) -> Group {
    if user_installed {
        return Group::Installed;
    }
    match rating.and_then(|r| r.level) {
        Some(Level::Recommended) => Group::Recommended,
        Some(Level::Advanced) => Group::Advanced,
        None => Group::NotRated,
        Some(Level::Expert) => Group::Expert,
        Some(Level::Unsafe) => Group::Unsafe,
    }
}
