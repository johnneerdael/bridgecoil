use adb_core::{
    backend::PocError,
    workspace::{
        bulk::Bulk,
        driver::{ShellDriver, ShellOutput},
        engine::{PackageAction, Workspace},
        history::History,
        types::*,
    },
};
use async_trait::async_trait;
use std::collections::VecDeque;
struct Driver {
    target: Target,
    answers: VecDeque<ShellOutput>,
    commands: Vec<String>,
}
#[async_trait]
impl ShellDriver for Driver {
    fn current_target(&self) -> Option<Target> {
        Some(self.target.clone())
    }
    async fn shell(&mut self, _: &Target, command: &str) -> Result<ShellOutput, PocError> {
        self.commands.push(command.into());
        Ok(self.answers.pop_front().expect("unexpected device command"))
    }
}
fn output(raw: &str, code: u8) -> ShellOutput {
    ShellOutput {
        stdout: raw.into(),
        stderr: String::new(),
        exit_code: Some(code),
    }
}
fn driver() -> Driver {
    Driver {
        target: Target {
            device_id: "d".into(),
            session_id: "s".into(),
            profile_id: 10,
        },
        answers: VecDeque::new(),
        commands: vec![],
    }
}
const BEFORE: &str = "versionCode=1\nUser 10: installed=true enabled=0 stopped=false";
const AFTER: &str = "versionCode=1\nUser 10: installed=true enabled=0 stopped=true";
#[tokio::test]
async fn bulk_stops_after_failure_and_records_each_result_without_claiming_a_transaction() {
    let dir = tempfile::tempdir().unwrap();
    let w = Workspace::new(History::open(&dir.path().join("db")).unwrap());
    let b = Bulk::default();
    let mut d = driver();
    let t = d.target.clone();
    d.answers = vec![
        output(BEFORE, 0),
        output(BEFORE, 0),
        output(BEFORE, 0),
        output(BEFORE, 0),
        output("", 0),
        output(AFTER, 0),
        output(BEFORE, 0),
        output("denied", 1),
    ]
    .into();
    let review = b
        .prepare(
            &w,
            &mut d,
            &t,
            vec![
                "org.one".into(),
                "org.two".into(),
                "org.three".into(),
                "bad;id".into(),
            ],
            PackageAction::ForceStop,
        )
        .await
        .unwrap();
    assert_eq!(review.excluded.len(), 1);
    let row = b.apply(&w, &mut d, &review.id, true).await.unwrap();
    assert_eq!(row.status, OperationStatus::Partial);
    assert!(row.output.contains("org.three"));
    assert!(
        !d.commands
            .iter()
            .any(|c| c == "am force-stop --user 10 'org.three'")
    );
    assert!(b.apply(&w, &mut d, &review.id, true).await.is_err());
    assert_eq!(w.history.list(0, 32).unwrap().total, 3);
}
#[tokio::test]
async fn cancellation_before_dispatch_changes_no_packages() {
    let dir = tempfile::tempdir().unwrap();
    let w = Workspace::new(History::open(&dir.path().join("db")).unwrap());
    let b = Bulk::default();
    let mut d = driver();
    d.answers = vec![output(BEFORE, 0)].into();
    let t = d.target.clone();
    let review = b
        .prepare(
            &w,
            &mut d,
            &t,
            vec!["org.one".into()],
            PackageAction::ForceStop,
        )
        .await
        .unwrap();
    b.cancel(&review.id).unwrap();
    let row = b.apply(&w, &mut d, &review.id, true).await.unwrap();
    assert_eq!(row.status, OperationStatus::Cancelled);
    assert_eq!(d.commands.len(), 1);
}

#[tokio::test]
async fn bulk_uninstalls_twenty_apps_once_and_excludes_system_and_current_home() {
    struct Tv {
        removed: std::collections::BTreeSet<String>,
        commands: Vec<String>,
    }
    #[async_trait]
    impl ShellDriver for Tv {
        fn current_target(&self) -> Option<Target> {
            Some(driver().target)
        }
        async fn shell(&mut self, _: &Target, command: &str) -> Result<ShellOutput, PocError> {
            self.commands.push(command.into());
            let package = command
                .split_whitespace()
                .last()
                .unwrap()
                .trim_matches('\'');
            let raw = if command.starts_with("pm uninstall ") {
                assert!(self.removed.insert(package.into()), "duplicate uninstall");
                "Success\n".into()
            } else if command.starts_with("dumpsys package ") {
                if self.removed.contains(package) {
                    format!("Unable to find package: {package}\n")
                } else {
                    let flags = if package == "org.system" {
                        "SYSTEM HAS_CODE"
                    } else {
                        "HAS_CODE"
                    };
                    format!(
                        "Package [{package}] (abc):\nversionCode=1\npkgFlags=[ {flags} ]\nUser 10: installed=true enabled=0 stopped=false\n"
                    )
                }
            } else {
                "org.home/.Main\n".into()
            };
            Ok(output(&raw, 0))
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let w = Workspace::new(History::open(&dir.path().join("db")).unwrap());
    let b = Bulk::default();
    let mut tv = Tv {
        removed: Default::default(),
        commands: vec![],
    };
    let mut packages: Vec<String> = (0..20).map(|i| format!("org.app{i}")).collect();
    packages.extend(["org.system".into(), "org.home".into(), "org.app0".into()]);
    let r = b
        .prepare(
            &w,
            &mut tv,
            &driver().target,
            packages,
            PackageAction::Uninstall,
        )
        .await
        .unwrap();
    assert_eq!(r.items.len(), 20);
    assert_eq!(r.excluded.len(), 2);
    assert!(
        r.excluded
            .iter()
            .any(|item| item.package == "org.system" && item.reason.contains("came with the TV"))
    );
    assert!(
        r.excluded
            .iter()
            .any(|item| item.package == "org.home" && item.reason.contains("Home app"))
    );
    assert!(tv.removed.is_empty(), "review must not mutate the TV");
    assert!(r.items.iter().all(|item| !item.review.reversible));
    let row = b.apply(&w, &mut tv, &r.id, true).await.unwrap();
    let results: serde_json::Value = serde_json::from_str(&row.output).unwrap();
    assert_eq!(tv.removed.len(), 20);
    assert!(
        results["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item["status"] == "verified")
    );
    assert_eq!(w.history.list(0, 32).unwrap().total, 21);
    assert!(b.apply(&w, &mut tv, &r.id, true).await.is_err());
}
