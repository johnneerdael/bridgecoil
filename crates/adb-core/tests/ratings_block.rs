use adb_core::{
    backend::PocError,
    ratings::{
        Level,
        fetch::{Commit, ListSource},
        store::Ratings,
    },
    workspace::{
        bulk::Bulk,
        driver::{ShellDriver, ShellOutput},
        engine::{Action, PackageAction, Workspace},
        history::History,
        types::Target,
    },
};
use async_trait::async_trait;
use serde_json::{Map, Value, json};
use std::sync::Arc;

const STATE: &str = "versionCode=3 minSdk=24\n User 0: installed=true hidden=false suspended=false stopped=false enabled=1\n";
struct Fake {
    commands: Vec<String>,
}
#[async_trait]
impl ShellDriver for Fake {
    fn current_target(&self) -> Option<Target> {
        Some(target())
    }
    async fn shell(&mut self, _: &Target, c: &str) -> Result<ShellOutput, PocError> {
        self.commands.push(c.into());
        let stdout = if c.starts_with("dumpsys package") {
            STATE.into()
        } else if c.contains("resolve-activity") {
            "com.home/.Main\n".to_owned()
        } else {
            String::new()
        };
        Ok(ShellOutput {
            stdout,
            stderr: String::new(),
            exit_code: Some(0),
        })
    }
}
fn fake() -> Fake {
    Fake { commands: vec![] }
}
fn target() -> Target {
    Target {
        device_id: "d".into(),
        session_id: "s".into(),
        profile_id: 0,
    }
}
struct Source;
impl ListSource for Source {
    fn latest_commit(&self) -> Result<Commit, PocError> {
        Ok(Commit {
            sha: "a".repeat(40),
            date: "2026-10-24T07:42:13Z".into(),
        })
    }
    fn download(&self, _: &str) -> Result<String, PocError> {
        let mut entries = Map::new();
        for index in 0..1000 {
            entries.insert(format!("com.a.{index}"), json!({"removal":"Recommended"}));
        }
        entries.insert("com.risky".into(), json!({"removal":"Unsafe"}));
        entries.insert("com.hard".into(), json!({"removal":"Expert"}));
        Ok(Value::Object(entries).to_string())
    }
}
fn rated(dir: &std::path::Path) -> Workspace {
    let ratings = Ratings::open(&dir.join("ratings"));
    ratings.update(&Source).unwrap();
    Workspace::new(History::open(&dir.join("ops.db")).unwrap()).with_ratings(Arc::new(ratings))
}
fn action(package: &str, action: PackageAction) -> Action {
    Action::Package {
        package: package.into(),
        action,
    }
}
const BLOCKED: [PackageAction; 3] = [
    PackageAction::Disable,
    PackageAction::Remove,
    PackageAction::ClearData,
];

#[tokio::test]
async fn unsafe_packages_refuse_the_three_actions() {
    let dir = tempfile::tempdir().unwrap();
    let w = rated(dir.path());
    for kind in BLOCKED {
        let mut d = fake();
        let e = w
            .prepare(&mut d, &target(), action("com.risky", kind))
            .await
            .unwrap_err();
        assert_eq!(e.kind, "unsafeRating");
        assert!(e.message.contains("Unsafe"));
        assert!(d.commands.is_empty());
    }
}

#[tokio::test]
async fn other_actions_stay_available_on_unsafe_packages() {
    let dir = tempfile::tempdir().unwrap();
    let w = rated(dir.path());
    for kind in [
        PackageAction::Enable,
        PackageAction::ForceStop,
        PackageAction::Restore,
        PackageAction::ClearCache,
        PackageAction::ResetEnabled,
    ] {
        let review = w
            .prepare(&mut fake(), &target(), action("com.risky", kind.clone()))
            .await;
        assert!(review.is_ok(), "{kind:?}");
    }
}

#[tokio::test]
async fn an_explicit_override_allows_the_action() {
    let dir = tempfile::tempdir().unwrap();
    let w = rated(dir.path());
    for kind in BLOCKED {
        let review = w
            .prepare_with(&mut fake(), &target(), action("com.risky", kind), true)
            .await
            .unwrap();
        assert_eq!(review.rating, Some(Level::Unsafe));
    }
}

#[tokio::test]
async fn expert_packages_are_not_blocked() {
    let dir = tempfile::tempdir().unwrap();
    let w = rated(dir.path());
    let review = w
        .prepare(
            &mut fake(),
            &target(),
            action("com.hard", PackageAction::Remove),
        )
        .await
        .unwrap();
    assert_eq!(review.rating, Some(Level::Expert));
}

#[tokio::test]
async fn unlisted_packages_have_no_rating_or_block() {
    let dir = tempfile::tempdir().unwrap();
    let review = rated(dir.path())
        .prepare(
            &mut fake(),
            &target(),
            action("com.other", PackageAction::Remove),
        )
        .await
        .unwrap();
    assert_eq!(review.rating, None);
}

#[tokio::test]
async fn no_list_means_no_block() {
    let dir = tempfile::tempdir().unwrap();
    let w = Workspace::new(History::open(&dir.path().join("ops.db")).unwrap());
    let review = w
        .prepare(
            &mut fake(),
            &target(),
            action("com.risky", PackageAction::Remove),
        )
        .await
        .unwrap();
    assert_eq!(review.rating, None);
}

#[tokio::test]
async fn removing_ratings_lifts_the_block() {
    let dir = tempfile::tempdir().unwrap();
    let ratings = Arc::new(Ratings::open(&dir.path().join("ratings")));
    ratings.update(&Source).unwrap();
    let w = Workspace::new(History::open(&dir.path().join("ops.db")).unwrap())
        .with_ratings(ratings.clone());
    ratings.remove().unwrap();
    assert!(
        w.prepare(
            &mut fake(),
            &target(),
            action("com.risky", PackageAction::Remove),
        )
        .await
        .is_ok()
    );
}

#[tokio::test]
async fn bulk_moves_blocked_packages_to_excluded_with_the_reason() {
    let dir = tempfile::tempdir().unwrap();
    let w = rated(dir.path());
    let review = Bulk::default()
        .prepare(
            &w,
            &mut fake(),
            &target(),
            vec!["com.risky".into(), "com.hard".into()],
            PackageAction::Disable,
        )
        .await
        .unwrap();
    assert_eq!(review.items.len(), 1);
    assert_eq!(review.items[0].package, "com.hard");
    assert_eq!(review.excluded.len(), 1);
    assert_eq!(review.excluded[0].package, "com.risky");
    assert!(review.excluded[0].reason.contains("Unsafe"));
    assert_eq!(review.excluded[0].rating, Some(Level::Unsafe));
}

#[tokio::test]
async fn bulk_restore_still_includes_unsafe_packages() {
    let dir = tempfile::tempdir().unwrap();
    let w = rated(dir.path());
    let review = Bulk::default()
        .prepare(
            &w,
            &mut fake(),
            &target(),
            vec!["com.risky".into()],
            PackageAction::Restore,
        )
        .await
        .unwrap();
    assert_eq!(review.items.len(), 1);
    assert!(review.excluded.is_empty());
}
