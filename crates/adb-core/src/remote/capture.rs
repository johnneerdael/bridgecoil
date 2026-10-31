use crate::backend::PocError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const MAX_CAPTURE_SECONDS: u32 = 20;

/// A physical button as the TV reports it. Nothing here is a guessed friendly name: `key` is a
/// Linux key name or a raw number, and `scan` tells apart buttons reported under the same name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyIdentity {
    pub source: String,
    pub key: String,
    pub scan: Option<String>,
}

pub fn capture_command(seconds: u32) -> Result<String, PocError> {
    if !(1..=MAX_CAPTURE_SECONDS).contains(&seconds) {
        return Err(PocError::new(
            "input",
            "Choose a listening time up to 20 seconds.",
        ));
    }
    Ok(format!("timeout {seconds} getevent -l 2>/dev/null; exit 0"))
}

fn is_button(code: &str) -> bool {
    let named = code.starts_with("KEY_")
        || (code.starts_with("BTN_") && code != "BTN_TOUCH" && !code.starts_with("BTN_TOOL"));
    named || (!code.is_empty() && code.bytes().all(|b| b.is_ascii_hexdigit()))
}

pub fn parse_capture(raw: &str) -> Vec<KeyIdentity> {
    let mut names = HashMap::new();
    let mut path = None;
    let mut scans: HashMap<&str, String> = HashMap::new();
    let mut presses: Vec<KeyIdentity> = Vec::new();
    for line in raw.lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        match fields.as_slice() {
            ["add", "device", _, device] => path = Some(*device),
            ["name:", ..] => {
                let name = line.split('"').nth(1).unwrap_or_default();
                if let Some(device) = path.take() {
                    names.insert(device, name.to_owned());
                }
            }
            [device, "EV_MSC", "MSC_SCAN", value] => {
                scans.insert(device.trim_end_matches(':'), value.to_lowercase());
            }
            [device, "EV_KEY", code, "DOWN" | "00000001"] => {
                let device = device.trim_end_matches(':');
                let scan = scans.remove(device);
                let Some(source) = names.get(device).filter(|_| is_button(code)) else {
                    continue;
                };
                let press = KeyIdentity {
                    source: source.clone(),
                    key: (*code).to_owned(),
                    scan,
                };
                if !presses.contains(&press) {
                    presses.push(press);
                }
            }
            _ => {}
        }
    }
    presses
}
