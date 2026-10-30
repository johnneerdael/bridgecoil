use crate::{backend::PocError, workspace::commands::package_arg};
use serde::{Deserialize, Serialize};
pub const CATALOG: &[(&str, &str)] = &[
    ("SYSTEM_ALERT_WINDOW", "Display over other apps"),
    ("WRITE_SETTINGS", "Modify system settings"),
    ("GET_USAGE_STATS", "Usage access"),
    ("RUN_IN_BACKGROUND", "Run in background"),
    ("RUN_ANY_IN_BACKGROUND", "Background restrictions"),
];
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mode {
    pub package_mode: String,
    pub uid_mode: Option<String>,
}
pub fn valid_mode(mode: &str) -> bool {
    ["allow", "ignore", "deny", "foreground", "default"].contains(&mode)
}
pub fn validate(op: &str) -> Result<(), PocError> {
    if CATALOG.iter().any(|(id, _)| *id == op) {
        Ok(())
    } else {
        Err(PocError::new(
            "input",
            "Choose a supported special-access policy.",
        ))
    }
}
pub fn command(user: u32, package: &str, op: &str, mode: &str) -> Result<String, PocError> {
    validate(op)?;
    if !valid_mode(mode) {
        return Err(PocError::new("input", "Choose a supported policy mode."));
    }
    Ok(format!(
        "cmd appops set --user {user} {} {op} {mode}",
        package_arg(package)?
    ))
}
pub fn probe_command(user: u32, package: &str, op: &str) -> Result<String, PocError> {
    validate(op)?;
    Ok(format!(
        "cmd appops get --user {user} {} {op}",
        package_arg(package)?
    ))
}
pub fn parse(raw: &str, op: &str) -> Result<Mode, PocError> {
    validate(op)?;
    let prefix = format!("{op}:");
    let mut package = None;
    let mut uid = None;
    let mut absent = false;
    for line in raw.lines().map(str::trim) {
        if line == "No operations." {
            absent = true;
        }
        let (line, is_uid) = if let Some(rest) = line.strip_prefix("Uid mode: ") {
            (rest, true)
        } else {
            (line, false)
        };
        if let Some(rest) = line.strip_prefix(&prefix) {
            let mode = rest.trim().split([';', ' ', '(']).next().unwrap_or("");
            if !valid_mode(mode) {
                continue;
            }
            let slot = if is_uid { &mut uid } else { &mut package };
            if slot.as_deref().is_some_and(|old| old != mode) {
                return Err(PocError::new(
                    "inspection",
                    "Conflicting app-op modes were reported.",
                ));
            }
            *slot = Some(mode.to_string());
        }
    }
    if package.is_none() && (absent || uid.is_some()) {
        package = Some("default".into());
    }
    Ok(Mode {
        package_mode: package.ok_or_else(|| {
            PocError::new(
                "inspection",
                "The TV did not report a readable special-access mode.",
            )
        })?,
        uid_mode: uid,
    })
}
