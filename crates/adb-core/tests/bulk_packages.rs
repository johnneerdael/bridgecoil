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
