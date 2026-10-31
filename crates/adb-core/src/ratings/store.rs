use super::{RatingList, check_acceptable, fetch::ListSource, parse_list};
use crate::{backend::PocError, workspace::history::now_ms};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, RwLock},
};

const LIST_FILE: &str = "uad_lists.json";
const META_FILE: &str = "uad_lists.meta.json";
const CHECK_INTERVAL_MS: i64 = 86_400_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RatingsStatus {
    pub stored: bool,
    pub commit: Option<String>,
    pub committed_at: Option<String>,
    pub packages: usize,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Meta {
    commit: String,
    committed_at: String,
    checked_ms: i64,
}
#[derive(Clone)]
struct Stored {
    list: Arc<RatingList>,
    meta: Meta,
}
pub struct Ratings {
    dir: PathBuf,
    stored: RwLock<Option<Stored>>,
    updating: Mutex<()>,
}

fn storage_error(e: impl std::fmt::Display) -> PocError {
    PocError::new("ratingsStorage", format!("Couldn't save ratings: {e}"))
}
fn write_atomic(dir: &Path, name: &str, data: &[u8]) -> Result<(), PocError> {
    let temp = dir.join(format!("{name}.tmp"));
    fs::create_dir_all(dir).map_err(storage_error)?;
    fs::write(&temp, data).map_err(storage_error)?;
    fs::rename(&temp, dir.join(name)).map_err(storage_error)
}
fn load(dir: &Path) -> Option<Stored> {
    let meta = serde_json::from_slice(&fs::read(dir.join(META_FILE)).ok()?).ok()?;
    let list = parse_list(&fs::read_to_string(dir.join(LIST_FILE)).ok()?).ok()?;
    (!list.is_empty()).then(|| Stored {
        list: Arc::new(list),
        meta,
    })
}

impl Ratings {
    pub fn open(dir: &Path) -> Self {
        Self {
            dir: dir.to_owned(),
            stored: RwLock::new(load(dir)),
            updating: Mutex::new(()),
        }
    }
    fn snapshot(&self) -> Option<Stored> {
        self.stored.read().ok()?.clone()
    }
    fn replace(&self, stored: Option<Stored>) -> Result<(), PocError> {
        *self.stored.write().map_err(storage_error)? = stored;
        Ok(())
    }
    pub fn current(&self) -> Option<Arc<RatingList>> {
        self.snapshot().map(|s| s.list)
    }
    pub fn status(&self) -> RatingsStatus {
        let stored = self.snapshot();
        RatingsStatus {
            stored: stored.is_some(),
            packages: stored.as_ref().map_or(0, |s| s.list.len()),
            commit: stored.as_ref().map(|s| s.meta.commit.clone()),
            committed_at: stored.map(|s| s.meta.committed_at),
        }
    }
    fn save_meta(&self, meta: &Meta) -> Result<(), PocError> {
        let data = serde_json::to_vec(meta).map_err(storage_error)?;
        write_atomic(&self.dir, META_FILE, &data)
    }
    fn mark_checked(&self, stored: &Stored, now: i64) -> Result<Stored, PocError> {
        let mut next = stored.clone();
        next.meta.checked_ms = now;
        self.save_meta(&next.meta)?;
        self.replace(Some(next.clone()))?;
        Ok(next)
    }
    fn update_at(&self, source: &dyn ListSource, now: i64) -> Result<bool, PocError> {
        let commit = source.latest_commit()?;
        let stored = self.snapshot();
        if let Some(stored) = stored.as_ref().filter(|s| s.meta.commit == commit.sha) {
            self.mark_checked(stored, now)?;
            return Ok(false);
        }
        let raw = source.download(&commit.sha)?;
        let list = parse_list(&raw)?;
        check_acceptable(&list, stored.as_ref().map(|s| s.list.as_ref()))?;
        let meta = Meta {
            commit: commit.sha,
            committed_at: commit.date,
            checked_ms: now,
        };
        write_atomic(&self.dir, LIST_FILE, raw.as_bytes())?;
        self.save_meta(&meta)?;
        self.replace(Some(Stored {
            list: Arc::new(list),
            meta,
        }))?;
        Ok(true)
    }
    pub fn update(&self, source: &dyn ListSource) -> Result<bool, PocError> {
        let _guard = self.updating.lock().map_err(storage_error)?;
        self.update_at(source, now_ms())
    }
    /// Records the attempt first, so a failing check still waits a day.
    pub fn update_if_due(&self, source: &dyn ListSource, now: i64) -> Result<bool, PocError> {
        let _guard = self.updating.lock().map_err(storage_error)?;
        let Some(stored) = self.snapshot() else {
            return Ok(false);
        };
        let since = now - stored.meta.checked_ms;
        if (0..CHECK_INTERVAL_MS).contains(&since) {
            return Ok(false);
        }
        self.mark_checked(&stored, now)?;
        self.update_at(source, now)
    }
    /// Waits for any update in flight, so a finishing download can't bring the list back.
    pub async fn remove_off_thread(self: Arc<Self>) -> Result<(), PocError> {
        tokio::task::spawn_blocking(move || self.remove())
            .await
            .map_err(storage_error)?
    }
    pub fn remove(&self) -> Result<(), PocError> {
        let _guard = self.updating.lock().map_err(storage_error)?;
        self.replace(None)?;
        for name in [META_FILE, LIST_FILE] {
            match fs::remove_file(self.dir.join(name)) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                    return Err(storage_error(e));
                }
                _ => {}
            }
        }
        Ok(())
    }
}
