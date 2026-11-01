use super::capture::{KEY_EVENTS, KeyIdentity};
use crate::{backend::PocError, shell_quote, workspace::commands::component_arg};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const RUNTIME_DIR: &str = "/data/local/tmp/bridgecoil";
const MAX_MAPPINGS: usize = 64;
const END: &str = "BRIDGECOIL_END_OF_FILE";
/// What Termux runs at boot. It lives here so that an app update can change it over ADB.
pub const BOOT_SCRIPT: &str = include_str!("scripts/boot.sh");
const BOOT_ID: &str = "/proc/sys/kernel/random/boot_id";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mapping {
    pub key: KeyIdentity,
    /// Holding the button rather than tapping it. A tap mapping also fires on a hold when
    /// nothing is mapped to holding, so a slow press never just does nothing.
    #[serde(default)]
    pub hold: bool,
    /// The app to open. Empty when the mapping sends a button instead.
    #[serde(default)]
    pub component: String,
    /// The button to send, from `SENDABLE`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub send: Option<String>,
    pub label: String,
}
/// Buttons a mapping can press. A fixed list, because these go straight to `input keyevent`.
pub const SENDABLE: &[&str] = &[
    "HOME",
    "BACK",
    "MENU",
    "VOLUME_UP",
    "VOLUME_DOWN",
    "VOLUME_MUTE",
    "MEDIA_PLAY_PAUSE",
    "MEDIA_NEXT",
    "MEDIA_PREVIOUS",
    "MEDIA_STOP",
    "MEDIA_REWIND",
    "MEDIA_FAST_FORWARD",
    "CHANNEL_UP",
    "CHANNEL_DOWN",
    "TV_INPUT",
    "CAPTIONS",
    "SETTINGS",
    "SEARCH",
    "SLEEP",
];
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MappingState {
    pub running: bool,
    pub paused: bool,
    /// Running in a later boot than the one mappings were saved in, so it restarted itself.
    pub survived_restart: bool,
    pub mappings: Vec<Mapping>,
    pub log: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum MappingChange {
    Apply { mappings: Vec<Mapping> },
    Pause { paused: bool },
    RemoveAll,
}
/// What the TV holds, without the log, which changes on its own between review and apply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MappingSnapshot {
    pub running: bool,
    pub paused: bool,
    pub mappings: Vec<Mapping>,
}
impl From<MappingState> for MappingSnapshot {
    fn from(state: MappingState) -> Self {
        Self {
            running: state.running,
            paused: state.paused,
            mappings: state.mappings,
        }
    }
}
pub struct MappingPlan {
    pub title: String,
    pub effect: String,
    pub losses: Vec<String>,
    pub command: String,
}
pub fn plan(change: &MappingChange) -> Result<MappingPlan, PocError> {
    Ok(match change {
        MappingChange::Apply { mappings } => MappingPlan {
            title: "Save button mappings".into(),
            effect: match mappings.len() {
                1 => "Run a small listener on the TV for 1 button.".into(),
                n => format!("Run a small listener on the TV for {n} buttons."),
            },
            losses: vec![
                "Each button still does its normal action first, then yours follows. To stop a button opening its original app, disable that app separately.".into(),
            ],
            command: apply_command(mappings)?,
        },
        MappingChange::Pause { paused: true } => MappingPlan {
            title: "Pause button mappings".into(),
            effect: "Buttons go back to their normal behaviour until you resume.".into(),
            losses: Vec::new(),
            command: pause_command(true),
        },
        MappingChange::Pause { paused: false } => MappingPlan {
            title: "Resume button mappings".into(),
            effect: "Your mapped buttons work again.".into(),
            losses: Vec::new(),
            command: pause_command(false),
        },
        MappingChange::RemoveAll => MappingPlan {
            title: "Remove all button mappings".into(),
            effect: "Stop the listener and delete everything Bridgecoil put in its own folder on the TV.".into(),
            losses: vec!["Every saved mapping on this TV is deleted.".into()],
            command: remove_command(),
        },
    })
}
pub fn verified(change: &MappingChange, after: &MappingSnapshot) -> bool {
    match change {
        MappingChange::Apply { mappings } => after.running && after.mappings == *mappings,
        MappingChange::Pause { paused } => after.paused == *paused,
        MappingChange::RemoveAll => !after.running && after.mappings.is_empty(),
    }
}

fn invalid() -> PocError {
    PocError::new("input", "This button mapping has an invalid value.")
}
fn valid_key(key: &KeyIdentity) -> bool {
    let token = !key.key.is_empty()
        && key.key.len() <= 40
        && key
            .key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_');
    let scan = key
        .scan
        .as_deref()
        .is_none_or(|s| !s.is_empty() && s.len() <= 16 && s.bytes().all(|b| b.is_ascii_hexdigit()));
    !key.source.is_empty()
        && key.source.len() <= 128
        && !key.source.contains(char::is_control)
        && token
        && scan
}
pub fn validate(mappings: &[Mapping]) -> Result<(), PocError> {
    if mappings.is_empty() || mappings.len() > MAX_MAPPINGS {
        return Err(PocError::new(
            "input",
            "Choose from 1 to 64 button mappings.",
        ));
    }
    let mut seen = HashSet::new();
    for mapping in mappings {
        match (&mapping.send, mapping.component.is_empty()) {
            (None, false) => {
                component_arg(&mapping.component).map_err(|_| invalid())?;
            }
            (Some(send), true) if SENDABLE.contains(&send.as_str()) => {}
            _ => return Err(invalid()),
        }
        if !valid_key(&mapping.key) {
            return Err(invalid());
        }
        if !seen.insert((&mapping.key, mapping.hold)) {
            return Err(PocError::new(
                "mappingConflict",
                "That button already has a mapping. Change it instead of adding another.",
            ));
        }
    }
    Ok(())
}

/// Every value is validated or quoted here, and the script never evaluates text.
pub fn listener_script(mappings: &[Mapping]) -> Result<String, PocError> {
    validate(mappings)?;
    let mut sources: Vec<&str> = Vec::new();
    for mapping in mappings {
        if !sources.contains(&mapping.key.source.as_str()) {
            sources.push(&mapping.key.source);
        }
    }
    let index = |source: &str| sources.iter().position(|s| *s == source).unwrap_or(0) + 1;
    let source_arms: Vec<_> = sources
        .iter()
        .enumerate()
        .map(|(i, source)| {
            format!(
                "        {}) paths=\"$paths$path:{} \" ;;",
                shell_quote(source),
                i + 1
            )
        })
        .collect();
    let path_arms: Vec<_> = (1..=sources.len())
        .map(|i| format!("        *\" $device:{i} \"*) source={i} ;;"))
        .collect();
    // The first matching arm wins, so hold mappings go ahead of the taps that also match holds.
    let ordered = mappings
        .iter()
        .filter(|m| m.hold)
        .chain(mappings.iter().filter(|m| !m.hold));
    let key_arms: Vec<_> = ordered
        .map(|m| {
            let source = index(&m.key.source);
            let gesture = if m.hold { "hold" } else { "*" };
            let pattern = match &m.key.scan {
                Some(scan) => format!("{gesture}:{source}:{scan}:*"),
                None => format!("{gesture}:{source}:*:{}", m.key.key),
            };
            let action = match &m.send {
                Some(send) => format!("send=KEYCODE_{send}"),
                None => format!("target={}", shell_quote(&m.component)),
            };
            format!("        {pattern}) {action} ;;")
        })
        .collect();
    Ok(include_str!("scripts/listener.sh")
        .replace("@@KEY_EVENTS@@", KEY_EVENTS)
        .replace("@@SOURCES@@", &source_arms.join("\n"))
        .replace("@@PATHS@@", &path_arms.join("\n"))
        .replace("@@KEYS@@", &key_arms.join("\n")))
}

/// Creates the folder, and writes this app's `boot.sh` into it.
pub fn write_boot_command() -> String {
    format!(
        "mkdir -p {RUNTIME_DIR} || exit 1\n\
         chmod 700 {RUNTIME_DIR}\n\
         cat > {RUNTIME_DIR}/boot.sh <<'{END}'\n{BOOT_SCRIPT}{END}\n\
         chmod 700 {RUNTIME_DIR}/boot.sh\n"
    )
}
pub fn apply_command(mappings: &[Mapping]) -> Result<String, PocError> {
    let script = listener_script(mappings)?;
    let stored =
        serde_json::to_string(mappings).map_err(|e| PocError::new("mapping", e.to_string()))?;
    Ok(format!(
        "{}\
         [ -f {RUNTIME_DIR}/listener.sh ] && sh {RUNTIME_DIR}/listener.sh --stop\n\
         cat > {RUNTIME_DIR}/listener.sh <<'{END}'\n{script}{END}\n\
         cat > {RUNTIME_DIR}/mappings.json <<'{END}'\n{stored}\n{END}\n\
         chmod 700 {RUNTIME_DIR}/listener.sh\n\
         cat {BOOT_ID} > {RUNTIME_DIR}/saved_boot 2>/dev/null\n\
         sh {RUNTIME_DIR}/listener.sh --start\n\
         sleep 1\n\
         exit 0",
        write_boot_command()
    ))
}
pub fn pause_command(paused: bool) -> String {
    if paused {
        format!("touch {RUNTIME_DIR}/paused")
    } else {
        format!("rm -f {RUNTIME_DIR}/paused")
    }
}
/// Removes only the folder Bridgecoil owns.
pub fn remove_command() -> String {
    format!(
        "[ -f {RUNTIME_DIR}/listener.sh ] && sh {RUNTIME_DIR}/listener.sh --stop\n\
         rm -rf {RUNTIME_DIR}\n\
         exit 0"
    )
}
pub fn state_command() -> String {
    format!(
        "running=\n\
         [ -x {RUNTIME_DIR}/listener.sh ] && sh {RUNTIME_DIR}/listener.sh --status && running=1\n\
         echo \"running=$running\"\n\
         [ -f {RUNTIME_DIR}/paused ] && echo paused=1 || echo paused=\n\
         echo \"boot=$(cat {BOOT_ID} 2>/dev/null)\"\n\
         echo \"saved=$(cat {RUNTIME_DIR}/saved_boot 2>/dev/null)\"\n\
         echo ---mappings\n\
         cat {RUNTIME_DIR}/mappings.json 2>/dev/null\n\
         echo ---log\n\
         tail -n 20 {RUNTIME_DIR}/listener.log 2>/dev/null\n\
         exit 0"
    )
}
pub fn parse_state(raw: &str) -> MappingState {
    let (head, rest) = raw.split_once("---mappings").unwrap_or((raw, ""));
    let (stored, log) = rest.split_once("---log").unwrap_or((rest, ""));
    let running = head.lines().any(|l| l.trim() == "running=1");
    let field = |name: &str| {
        head.lines()
            .find_map(|l| l.trim().strip_prefix(name))
            .unwrap_or("")
    };
    let (boot, saved) = (field("boot="), field("saved="));
    MappingState {
        running,
        paused: head.lines().any(|l| l.trim() == "paused=1"),
        survived_restart: running && !boot.is_empty() && !saved.is_empty() && boot != saved,
        mappings: serde_json::from_str(stored.trim()).unwrap_or_default(),
        log: log
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_owned)
            .collect(),
    }
}
