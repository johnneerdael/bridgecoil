use super::types::*;
use crate::backend::PocError;
use rusqlite::{Connection, params};
use std::{path::Path, sync::Mutex};
fn storage(e: impl std::fmt::Display) -> PocError {
    PocError::new("storage", e.to_string())
}
pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|v| v.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}
pub struct History {
    connection: Mutex<Connection>,
}
impl History {
    pub fn open(path: &Path) -> Result<Self, PocError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(storage)?;
        }
        let mut connection = Connection::open(path).map_err(storage)?;
        connection
            .busy_timeout(std::time::Duration::from_secs(3))
            .map_err(storage)?;
        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(storage)?;
        if version > 1 {
            return Err(PocError::new(
                "storage",
                "Operation history belongs to a newer app version. Update the app; the file has not been changed.",
            ));
        }
        let tx = connection.transaction().map_err(storage)?;
        tx.execute_batch("CREATE TABLE IF NOT EXISTS operations(id TEXT PRIMARY KEY,created_ms INTEGER NOT NULL,status TEXT NOT NULL,record TEXT NOT NULL); CREATE INDEX IF NOT EXISTS operations_created ON operations(created_ms DESC); PRAGMA user_version=1;").map_err(storage)?;
        tx.commit().map_err(storage)?;
        let h = Self {
            connection: Mutex::new(connection),
        };
        // A process restart cannot establish the outcome of a mutation that was in flight.
        let interrupted = {
            let db = h.connection.lock().map_err(storage)?;
            let mut q = db
                .prepare("SELECT record FROM operations WHERE status IN ('queued','running')")
                .map_err(storage)?;
            q.query_map([], |r| r.get::<_, String>(0))
                .map_err(storage)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(storage)?
        };
        for raw in interrupted {
            let mut row: OperationRecord = serde_json::from_str(&raw).map_err(storage)?;
            row.status = OperationStatus::Unknown;
            row.updated_ms = now_ms();
            row.error=Some("The app stopped before this operation's result was recorded. Inspect the device before retrying.".into());
            h.put(&row)?;
        }
        Ok(h)
    }
    pub fn put(&self, row: &OperationRecord) -> Result<(), PocError> {
        let mut row = row.clone();
        if row.output.len() > 65536 {
            let mut end = 65536;
            while !row.output.is_char_boundary(end) {
                end -= 1;
            }
            row.output.truncate(end);
            row.output
                .push_str("\n[History keeps the first 64 KB of this output.]");
        }
        let status = serde_json::to_value(&row.status)
            .map_err(storage)?
            .as_str()
            .ok_or_else(|| storage("Invalid operation status"))?
            .to_owned();
        let raw = serde_json::to_string(&row).map_err(storage)?;
        let db = self.connection.lock().map_err(storage)?;
        db.execute("INSERT INTO operations(id,created_ms,status,record)VALUES(?1,?2,?3,?4)ON CONFLICT(id)DO UPDATE SET status=excluded.status,record=excluded.record",params![row.id,row.created_ms,status,raw]).map_err(storage)?;
        Ok(())
    }
    pub fn get(&self, id: &str) -> Result<OperationRecord, PocError> {
        let db = self.connection.lock().map_err(storage)?;
        let raw: String = db
            .query_row("SELECT record FROM operations WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .map_err(storage)?;
        serde_json::from_str(&raw).map_err(storage)
    }
    pub fn list(&self, page: usize, page_size: usize) -> Result<Page<OperationRecord>, PocError> {
        if !(1..=64).contains(&page_size) || page > 1_000_000 {
            return Err(PocError::new(
                "input",
                "History page size must be between 1 and 64.",
            ));
        }
        let offset = page
            .checked_mul(page_size)
            .ok_or_else(|| storage("Page offset overflow"))?;
        let db = self.connection.lock().map_err(storage)?;
        let total: i64 = db
            .query_row("SELECT COUNT(*) FROM operations", [], |r| r.get(0))
            .map_err(storage)?;
        let mut q = db
            .prepare(
                "SELECT record FROM operations ORDER BY created_ms DESC,id DESC LIMIT ?1 OFFSET ?2",
            )
            .map_err(storage)?;
        let raw = q
            .query_map(params![page_size as i64, offset as i64], |r| {
                r.get::<_, String>(0)
            })
            .map_err(storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage)?;
        let items = raw
            .iter()
            .map(|s| serde_json::from_str(s).map_err(storage))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Page {
            items,
            total: total as usize,
            page,
            page_size,
        })
    }
    pub fn clear(&self) -> Result<(), PocError> {
        self.connection
            .lock()
            .map_err(storage)?
            .execute("DELETE FROM operations", [])
            .map_err(storage)?;
        Ok(())
    }
}
