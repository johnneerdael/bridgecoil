use adb_core::{
    backend::PocError,
    ratings::{
        fetch::{Commit, ListSource, list_url, parse_commit},
        store::Ratings,
    },
    workspace::history::now_ms,
};
use serde_json::{Map, Value, json};
use std::{
    cell::Cell,
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};

const DAY_MS: i64 = 86_400_000;
const SHA_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SHA_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn good_list(count: usize) -> String {
    let mut entries = Map::new();
    for index in 0..count {
        entries.insert(format!("com.a.{index}"), json!({"removal":"Recommended"}));
    }
    entries.insert("com.risky".into(), json!({"removal":"Unsafe"}));
    Value::Object(entries).to_string()
}
struct FakeSource {
    sha: &'static str,
    body: String,
    fail_check: bool,
    fail_download: bool,
    checks: Cell<u32>,
    downloads: Cell<u32>,
}
impl FakeSource {
    fn new(sha: &'static str, body: String) -> Self {
        Self {
            sha,
            body,
            fail_check: false,
            fail_download: false,
            checks: Cell::new(0),
            downloads: Cell::new(0),
        }
    }
}
impl ListSource for FakeSource {
    fn latest_commit(&self) -> Result<Commit, PocError> {
        self.checks.set(self.checks.get() + 1);
        if self.fail_check {
            return Err(PocError::new("ratingsNetwork", "offline"));
        }
        Ok(Commit {
            sha: self.sha.into(),
            date: "2026-10-24T07:42:13Z".into(),
        })
    }
    fn download(&self, sha: &str) -> Result<String, PocError> {
        assert_eq!(sha, self.sha);
        self.downloads.set(self.downloads.get() + 1);
        if self.fail_download {
            return Err(PocError::new("ratingsNetwork", "offline"));
        }
        Ok(self.body.clone())
    }
}
fn stored(dir: &std::path::Path) -> Ratings {
    let ratings = Ratings::open(dir);
    assert!(
        ratings
            .update(&FakeSource::new(SHA_A, good_list(1000)))
            .unwrap()
    );
    ratings
}
fn unchanged(ratings: &Ratings) {
    assert_eq!(ratings.status().commit.as_deref(), Some(SHA_A));
    assert_eq!(ratings.status().packages, 1001);
    assert!(ratings.current().unwrap().get("com.risky").is_some());
}

#[test]
fn first_update_stores_the_list_and_commit() {
    let dir = tempfile::tempdir().unwrap();
    let status = stored(dir.path()).status();
    assert!(status.stored);
    assert_eq!(status.commit.as_deref(), Some(SHA_A));
    assert_eq!(status.committed_at.as_deref(), Some("2026-10-24T07:42:13Z"));
    assert_eq!(status.packages, 1001);
}

#[test]
fn nothing_is_stored_before_the_first_update() {
    let dir = tempfile::tempdir().unwrap();
    let ratings = Ratings::open(dir.path());
    assert!(!ratings.status().stored);
    assert!(ratings.current().is_none());
}

#[test]
fn a_new_instance_reloads_what_was_stored() {
    let dir = tempfile::tempdir().unwrap();
    stored(dir.path());
    unchanged(&Ratings::open(dir.path()));
}

#[test]
fn same_commit_downloads_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let ratings = stored(dir.path());
    let source = FakeSource::new(SHA_A, good_list(1000));
    assert!(!ratings.update(&source).unwrap());
    assert_eq!(source.downloads.get(), 0);
}

#[test]
fn a_new_commit_replaces_the_list() {
    let dir = tempfile::tempdir().unwrap();
    let ratings = stored(dir.path());
    assert!(
        ratings
            .update(&FakeSource::new(SHA_B, good_list(1200)))
            .unwrap()
    );
    assert_eq!(ratings.status().commit.as_deref(), Some(SHA_B));
    assert_eq!(ratings.status().packages, 1201);
}

#[test]
fn failed_check_keeps_the_stored_list() {
    let dir = tempfile::tempdir().unwrap();
    let ratings = stored(dir.path());
    let mut source = FakeSource::new(SHA_B, good_list(1000));
    source.fail_check = true;
    assert!(ratings.update(&source).is_err());
    unchanged(&ratings);
}

#[test]
fn failed_download_keeps_the_stored_list() {
    let dir = tempfile::tempdir().unwrap();
    let ratings = stored(dir.path());
    let mut source = FakeSource::new(SHA_B, good_list(1000));
    source.fail_download = true;
    assert!(ratings.update(&source).is_err());
    unchanged(&ratings);
}

#[test]
fn invalid_json_keeps_the_stored_list() {
    let dir = tempfile::tempdir().unwrap();
    let ratings = stored(dir.path());
    assert!(
        ratings
            .update(&FakeSource::new(SHA_B, "{oops".into()))
            .is_err()
    );
    unchanged(&ratings);
    unchanged(&Ratings::open(dir.path()));
}

#[test]
fn a_rejected_list_keeps_the_stored_list() {
    let dir = tempfile::tempdir().unwrap();
    let ratings = stored(dir.path());
    let e = ratings
        .update(&FakeSource::new(SHA_B, good_list(5)))
        .unwrap_err();
    assert_eq!(e.kind, "ratingsRejected");
    unchanged(&ratings);
}

#[test]
fn a_rejected_list_is_never_stored_first_time() {
    let dir = tempfile::tempdir().unwrap();
    let ratings = Ratings::open(dir.path());
    assert!(
        ratings
            .update(&FakeSource::new(SHA_A, good_list(5)))
            .is_err()
    );
    assert!(!ratings.status().stored);
}

#[test]
fn corrupt_stored_file_counts_as_no_list() {
    let dir = tempfile::tempdir().unwrap();
    stored(dir.path());
    std::fs::write(dir.path().join("uad_lists.json"), "{oops").unwrap();
    let ratings = Ratings::open(dir.path());
    assert!(!ratings.status().stored);
    assert!(ratings.current().is_none());
}

#[test]
fn remove_clears_memory_and_disk() {
    let dir = tempfile::tempdir().unwrap();
    let ratings = stored(dir.path());
    ratings.remove().unwrap();
    assert!(ratings.current().is_none());
    assert!(!Ratings::open(dir.path()).status().stored);
}

#[test]
fn removing_when_nothing_is_stored_is_fine() {
    let dir = tempfile::tempdir().unwrap();
    assert!(Ratings::open(dir.path()).remove().is_ok());
}

#[test]
fn due_check_needs_a_stored_list() {
    let dir = tempfile::tempdir().unwrap();
    let source = FakeSource::new(SHA_A, good_list(1000));
    assert!(
        !Ratings::open(dir.path())
            .update_if_due(&source, now_ms())
            .unwrap()
    );
    assert_eq!(source.checks.get(), 0);
    assert_eq!(source.downloads.get(), 0);
}

#[test]
fn due_check_runs_once_a_day() {
    let dir = tempfile::tempdir().unwrap();
    let ratings = stored(dir.path());
    let source = FakeSource::new(SHA_A, good_list(1000));
    let start = now_ms();
    ratings.update_if_due(&source, start + 1000).unwrap();
    assert_eq!(source.checks.get(), 0);
    ratings.update_if_due(&source, start + DAY_MS).unwrap();
    assert_eq!(source.checks.get(), 1);
    ratings
        .update_if_due(&source, start + DAY_MS + 23 * 3_600_000)
        .unwrap();
    assert_eq!(source.checks.get(), 1);
    ratings.update_if_due(&source, start + 2 * DAY_MS).unwrap();
    assert_eq!(source.checks.get(), 2);
}

#[test]
fn failed_due_check_still_waits_a_day() {
    let dir = tempfile::tempdir().unwrap();
    let ratings = stored(dir.path());
    let mut source = FakeSource::new(SHA_B, good_list(1000));
    source.fail_check = true;
    let start = now_ms() + DAY_MS;
    assert!(ratings.update_if_due(&source, start).is_err());
    assert!(!ratings.update_if_due(&source, start + 1000).unwrap());
    assert_eq!(source.checks.get(), 1);
}

#[test]
fn a_clock_moved_backwards_does_not_stop_checks() {
    let dir = tempfile::tempdir().unwrap();
    let ratings = stored(dir.path());
    let source = FakeSource::new(SHA_A, good_list(1000));
    ratings
        .update_if_due(&source, now_ms() - 30 * DAY_MS)
        .unwrap();
    assert_eq!(source.checks.get(), 1);
}

#[test]
fn reads_the_commit_github_returns() {
    let commit = parse_commit(&format!(
        r#"[{{"sha":"{SHA_A}","commit":{{"committer":{{"date":"2026-10-24T07:42:13Z"}}}}}}]"#
    ))
    .unwrap();
    assert_eq!(commit.sha, SHA_A);
    assert_eq!(commit.date, "2026-10-24T07:42:13Z");
}

#[test]
fn a_rate_limit_reply_is_a_failed_check() {
    let body = r#"{"message":"API rate limit exceeded"}"#;
    assert_eq!(parse_commit(body).unwrap_err().kind, "ratingsNetwork");
    assert!(parse_commit("[]").is_err());
    assert!(parse_commit("<html>").is_err());
}

#[test]
fn a_hash_must_be_forty_hex_characters() {
    for sha in ["abc", "../../etc/passwd", &"g".repeat(40)] {
        let body = format!(r#"[{{"sha":"{sha}","commit":{{"committer":{{"date":"d"}}}}}}]"#);
        assert!(parse_commit(&body).is_err(), "{sha}");
    }
}

#[test]
fn downloads_are_pinned_to_the_commit() {
    assert_eq!(
        list_url(SHA_A),
        format!(
            "https://raw.githubusercontent.com/Universal-Debloater-Alliance/universal-android-debloater-next-generation/{SHA_A}/resources/assets/uad_lists.json"
        )
    );
}

struct GatedSource {
    entered: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
}
impl ListSource for GatedSource {
    fn latest_commit(&self) -> Result<Commit, PocError> {
        self.entered.send(()).unwrap();
        self.release.recv().unwrap();
        Ok(Commit {
            sha: SHA_A.into(),
            date: "2026-10-24T07:42:13Z".into(),
        })
    }
    fn download(&self, _: &str) -> Result<String, PocError> {
        Ok(good_list(1000))
    }
}

#[tokio::test]
async fn removing_during_an_update_leaves_the_async_runtime_free() {
    let dir = tempfile::tempdir().unwrap();
    let ratings = Arc::new(stored(dir.path()));
    let (entered, reached) = mpsc::channel();
    let (release, gate) = mpsc::channel();
    let updating = ratings.clone();
    let update = std::thread::spawn(move || {
        updating.update(&GatedSource {
            entered,
            release: gate,
        })
    });
    reached.recv().unwrap();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(600));
        release.send(()).unwrap();
    });
    let removal = tokio::spawn(ratings.clone().remove_off_thread());
    let start = Instant::now();
    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(start.elapsed() < Duration::from_millis(300));
    removal.await.unwrap().unwrap();
    update.join().unwrap().unwrap();
    assert!(ratings.current().is_none());
}
