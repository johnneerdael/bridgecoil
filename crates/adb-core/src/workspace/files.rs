use super::commands::remote_path;
use crate::{backend::PocError, shell_quote};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

/// Largest folder returned in one listing; the rest is reported as truncated.
pub const MAX_ENTRIES: usize = 5000;
const FLAGS: usize = 6;
const META: &str = ":meta";
const MAX_NAME_BYTES: usize = 255;
const CANCEL_POLL: Duration = Duration::from_millis(100);
/// Removing or replacing these would take shared storage or app data with them.
const PROTECTED: [&str; 9] = [
    "/",
    "/sdcard",
    "/sdcard/Android",
    "/storage",
    "/storage/emulated",
    "/storage/emulated/0",
    "/storage/emulated/0/Android",
    "/storage/self",
    "/storage/self/primary",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FolderStatus {
    Ok,
    Denied,
    Missing,
    NotDirectory,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FileKind {
    Folder,
    File,
    Other,
    BrokenLink,
    /// Listed by the folder, but Android refuses to say anything else about it.
    Denied,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub kind: FileKind,
    pub link: bool,
    pub can_open: bool,
    pub readable: bool,
    pub size: Option<u64>,
    pub modified: Option<i64>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileListing {
    pub path: String,
    pub status: FolderStatus,
    pub writable: bool,
    pub entries: Vec<FileEntry>,
    pub total: usize,
    pub truncated: bool,
}

pub fn listing_command(path: &str) -> String {
    format!(
        "set -- {}\n{}",
        shell_quote(path),
        include_str!("list_files.sh")
    )
}

/// Maps name -> (size, modified). Lines start at a `BC|` marker so a newline in a name stays in that name.
fn parse_meta(meta: &str) -> HashMap<&str, (u64, i64)> {
    meta.strip_prefix("BC|")
        .into_iter()
        .flat_map(|rest| rest.split("\nBC|"))
        .filter_map(|line| {
            let mut fields = line.trim_end_matches('\n').splitn(3, '|');
            let size = fields.next()?.parse().ok()?;
            let modified = fields.next()?.parse().ok()?;
            let name = fields.next()?.strip_prefix("./")?;
            Some((name, (size, modified)))
        })
        .collect()
}

fn parse_entry(folder: &str, record: &str, meta: &HashMap<&str, (u64, i64)>) -> Option<FileEntry> {
    if !record.is_char_boundary(FLAGS) {
        return None;
    }
    let (flags, name) = record.split_at(FLAGS);
    if name.is_empty() {
        return None;
    }
    let flag = |index: usize, value: u8| flags.as_bytes()[index] == value;
    let exists = flag(1, b'e');
    let link = flag(2, b'l');
    let kind = match (exists, flags.as_bytes()[0]) {
        (true, b'd') => FileKind::Folder,
        (true, b'f') => FileKind::File,
        (true, _) => FileKind::Other,
        (false, _) if link => FileKind::BrokenLink,
        (false, _) => FileKind::Denied,
    };
    let readable = flag(3, b'r');
    let details = meta.get(name);
    Some(FileEntry {
        path: format!("{}/{name}", folder.trim_end_matches('/')),
        name: name.to_owned(),
        kind,
        link,
        can_open: kind == FileKind::Folder && readable && flag(5, b'x'),
        readable,
        size: details.map(|d| d.0),
        modified: details.map(|d| d.1),
    })
}

/// Folders first, then by name ignoring case, with hidden names last in each group.
fn sort_key(entry: &FileEntry) -> (bool, bool, String) {
    (
        entry.kind != FileKind::Folder,
        entry.name.starts_with('.'),
        entry.name.to_lowercase(),
    )
}

pub fn parse_listing(path: &str, raw: &str) -> Result<FileListing, PocError> {
    let mut records = raw.split('\0');
    let status = match records.next().unwrap_or_default() {
        "ok" => FolderStatus::Ok,
        "denied" => FolderStatus::Denied,
        "missing" => FolderStatus::Missing,
        "notDirectory" => FolderStatus::NotDirectory,
        _ => {
            return Err(PocError::new(
                "deviceCommand",
                "This TV did not return a readable folder listing.",
            ));
        }
    };
    let writable = status == FolderStatus::Ok && records.next() == Some("w");
    let names: Vec<_> = records.by_ref().take_while(|r| *r != META).collect();
    let meta = parse_meta(records.next().unwrap_or_default());
    let mut entries: Vec<_> = names
        .iter()
        .filter_map(|record| parse_entry(path, record, &meta))
        .collect();
    entries.sort_by_cached_key(sort_key);
    let total = entries.len();
    entries.truncate(MAX_ENTRIES);
    Ok(FileListing {
        path: path.to_owned(),
        status,
        writable,
        entries,
        total,
        truncated: total > MAX_ENTRIES,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathState {
    pub path: String,
    pub canonical: String,
    pub exists: bool,
    pub parent_writable: bool,
    pub kind: String,
    pub size: u64,
    pub modified: i64,
    pub items: u64,
}

pub fn state_command(paths: &[&str]) -> String {
    let quoted: Vec<_> = paths.iter().map(|p| shell_quote(p)).collect();
    format!(
        "set -- {}\n{}",
        quoted.join(" "),
        include_str!("file_state.sh")
    )
}

/// Sorted by path, so a move and its undo describe the same state.
pub fn parse_states(raw: &str) -> Result<Vec<PathState>, PocError> {
    let invalid = || PocError::new("inspection", "The TV did not report this file's state.");
    let records: Vec<_> = raw.split('\0').collect();
    let mut states = Vec::new();
    for [details, path, canonical] in records.as_chunks::<3>().0 {
        let fields: Vec<_> = details.splitn(5, '|').collect();
        let [flags, kind, size, modified, items] = fields[..] else {
            return Err(invalid());
        };
        states.push(PathState {
            path: (*path).to_owned(),
            canonical: remote_path(canonical).map_err(|_| invalid())?,
            exists: flags.starts_with('e'),
            parent_writable: flags.ends_with('w'),
            kind: kind.to_owned(),
            size: size.parse().unwrap_or_default(),
            modified: modified.parse().unwrap_or_default(),
            items: items.trim().parse().unwrap_or_default(),
        });
    }
    if states.is_empty() {
        return Err(invalid());
    }
    states.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(states)
}

/// Actions carry the exact path shown in review, so anything needing normalising is refused.
pub fn action_path(path: &str) -> Result<&str, PocError> {
    if remote_path(path)? != path {
        return Err(PocError::new("input", "Choose an absolute device path."));
    }
    Ok(path)
}

pub fn valid_name(name: &str) -> bool {
    !name.trim().is_empty()
        && name.len() <= MAX_NAME_BYTES
        && name != "."
        && name != ".."
        && !name.contains(['/', '\0'])
}

pub fn child_path(folder: &str, name: &str) -> Result<String, PocError> {
    if !valid_name(name) {
        return Err(PocError::new("input", "Choose a valid file name."));
    }
    let folder = remote_path(folder)?;
    Ok(format!("{}/{name}", folder.trim_end_matches('/')))
}

/// Best guess at a picked file's name; Android content URLs may only hold an opaque ID.
pub fn picked_name(uri: &str) -> String {
    let bytes = uri.split(['?', '#']).next().unwrap_or_default().as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let hex = (bytes[index] == b'%')
            .then(|| bytes.get(index + 1..index + 3))
            .flatten()
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        decoded.push(hex.unwrap_or(bytes[index]));
        index += if hex.is_some() { 3 } else { 1 };
    }
    let text = String::from_utf8_lossy(&decoded);
    let name = text.rsplit(['/', ':']).next().unwrap_or_default();
    if valid_name(name) {
        name.to_owned()
    } else {
        "upload".to_owned()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interrupted {
    Cancelled,
    Stalled,
}

/// Waits for `work`, noticing a cancel request even while the TV has stopped responding.
pub async fn interruptible<T>(
    work: impl Future<Output = T>,
    cancel: &AtomicBool,
    stall: Duration,
) -> Result<T, Interrupted> {
    let deadline = Instant::now() + stall;
    tokio::pin!(work);
    loop {
        if cancel.load(Ordering::SeqCst) {
            return Err(Interrupted::Cancelled);
        }
        if Instant::now() >= deadline {
            return Err(Interrupted::Stalled);
        }
        if let Ok(output) = tokio::time::timeout(CANCEL_POLL, &mut work).await {
            return Ok(output);
        }
    }
}

pub fn after_disconnect(error: PocError) -> PocError {
    PocError::new(
        &error.kind,
        format!(
            "{} The TV was disconnected, so reconnect to continue.",
            error.message
        ),
    )
}

/// Android's sync service reports a refused name or location with this text.
pub fn upload_failure(error: PocError) -> PocError {
    if error.message.contains("couldn't create file") {
        return PocError::new("permission", "The TV refused this file name or location.");
    }
    error
}

fn protect(state: &PathState) -> Result<(), PocError> {
    if PROTECTED.contains(&state.path.as_str()) || PROTECTED.contains(&state.canonical.as_str()) {
        return Err(PocError::new(
            "protected",
            "This folder is protected, because shared storage depends on it.",
        ));
    }
    Ok(())
}
fn require_writable(state: &PathState) -> Result<(), PocError> {
    if !state.parent_writable {
        return Err(PocError::new(
            "permission",
            "Android doesn't allow ADB to change this folder.",
        ));
    }
    Ok(())
}
fn require_exists(state: &PathState) -> Result<(), PocError> {
    if !state.exists {
        return Err(PocError::new("staleReview", "This item no longer exists."));
    }
    Ok(())
}
fn require_free(state: &PathState) -> Result<(), PocError> {
    if state.exists {
        return Err(PocError::new(
            "exists",
            "An item with this name already exists here.",
        ));
    }
    Ok(())
}
pub fn check_delete(state: &PathState) -> Result<(), PocError> {
    require_exists(state)?;
    protect(state)?;
    require_writable(state)
}
pub fn check_move(from: &PathState, to: &PathState) -> Result<(), PocError> {
    require_exists(from)?;
    protect(from)?;
    require_writable(from)?;
    require_free(to)?;
    require_writable(to)?;
    if to.canonical.starts_with(&format!("{}/", from.canonical)) {
        return Err(PocError::new(
            "input",
            "A folder can't be moved into itself.",
        ));
    }
    Ok(())
}
pub fn check_create(state: &PathState) -> Result<(), PocError> {
    require_free(state)?;
    require_writable(state)
}
pub fn state_of<'a>(states: &'a [PathState], path: &str) -> Result<&'a PathState, PocError> {
    states
        .iter()
        .find(|s| s.path == path)
        .ok_or_else(|| PocError::new("inspection", "The TV did not report this file's state."))
}
fn is_folder(state: &PathState) -> bool {
    state.kind == "directory"
}
fn item_count(items: u64) -> String {
    match items {
        1 => "1 item".to_owned(),
        n => format!("{n} items"),
    }
}
pub fn delete_loss(state: &PathState) -> String {
    if state.kind == "symbolic link" {
        "Only the link is removed. The item it points to stays.".into()
    } else if is_folder(state) {
        format!(
            "This folder and {} inside it will be permanently deleted. The TV has no trash.",
            item_count(state.items)
        )
    } else {
        "This file will be permanently deleted. The TV has no trash.".into()
    }
}
pub fn state_summary(states: &[PathState]) -> String {
    match states.iter().find(|s| s.exists) {
        Some(s) if is_folder(s) => format!("Folder with {}.", item_count(s.items)),
        Some(s) if s.kind == "symbolic link" => "Link.".into(),
        Some(s) => format!("File of {} bytes.", s.size),
        None => "Nothing exists at this path yet.".into(),
    }
}
