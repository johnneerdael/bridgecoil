use adb_core::{
    backend::PocError,
    workspace::{
        driver::{ShellDriver, ShellOutput},
        engine::{Action, PackageAction, Workspace},
        history::History,
        types::*,
    },
};
use async_trait::async_trait;
use std::collections::VecDeque;
struct Fake {
    target: Target,
    answers: VecDeque<Result<ShellOutput, PocError>>,
    commands: Vec<String>,
}
#[async_trait]
impl ShellDriver for Fake {
    fn current_target(&self) -> Option<Target> {
        Some(self.target.clone())
    }
    async fn shell(&mut self, _: &Target, c: &str) -> Result<ShellOutput, PocError> {
        self.commands.push(c.into());
        self.answers.pop_front().expect("unexpected command")
    }
}
fn ok(s: &str) -> Result<ShellOutput, PocError> {
    Ok(ShellOutput {
        stdout: s.into(),
        stderr: String::new(),
        exit_code: Some(0),
    })
}
fn target() -> Target {
    Target {
        device_id: "d".into(),
        session_id: "s".into(),
        profile_id: 10,
    }
}
fn fake(answers: Vec<Result<ShellOutput, PocError>>) -> Fake {
    Fake {
        target: target(),
        answers: answers.into(),
        commands: vec![],
    }
}
fn workspace(dir: &std::path::Path) -> Workspace {
    Workspace::new(History::open(&dir.join("ops.db")).unwrap())
}
const ENABLED: &str = "versionCode=3 minSdk=24\n User 10: installed=true hidden=false suspended=false stopped=false enabled=0\n";
const DISABLED: &str = "versionCode=3 minSdk=24\n User 10: installed=true hidden=false suspended=false stopped=false enabled=3\n";
#[tokio::test]
async fn changed_profile_rejects_review_without_mutation() {
    let dir = tempfile::tempdir().unwrap();
    let w = workspace(dir.path());
    let mut d = fake(vec![ok(ENABLED), ok("org.home/.Main")]);
    let plan = w
        .prepare(
            &mut d,
            &target(),
            Action::Package {
                package: "org.demo".into(),
                action: PackageAction::Disable,
            },
        )
        .await
        .unwrap();
    d.target.profile_id = 0;
    assert!(w.apply(&mut d, &plan.id).await.is_err());
    assert_eq!(d.commands.len(), 2);
}
#[tokio::test]
async fn state_changes_require_a_new_review() {
    let dir = tempfile::tempdir().unwrap();
    let w = workspace(dir.path());
    let mut d = fake(vec![ok(ENABLED), ok("org.home/.Main"), ok(DISABLED)]);
    let p = w
        .prepare(
            &mut d,
            &target(),
            Action::Package {
                package: "org.demo".into(),
                action: PackageAction::Disable,
            },
        )
        .await
        .unwrap();
    assert!(w.apply(&mut d, &p.id).await.is_err());
    assert_eq!(d.commands.len(), 3);
}
#[tokio::test]
async fn disable_verifies_profile_and_has_reviewable_inverse() {
    let dir = tempfile::tempdir().unwrap();
    let w = workspace(dir.path());
    let mut d = fake(vec![
        ok(ENABLED),
        ok("org.home/.Main"),
        ok(ENABLED),
        ok("org.home/.Main"),
        ok("Package org.demo new state: disabled-user"),
        ok(DISABLED),
    ]);
    let p = w
        .prepare(
            &mut d,
            &target(),
            Action::Package {
                package: "org.demo".into(),
                action: PackageAction::Disable,
            },
        )
        .await
        .unwrap();
    let r = w.apply(&mut d, &p.id).await.unwrap();
    assert_eq!(r.status, OperationStatus::Verified);
    assert!(r.recovery.is_some());
    assert!(
        d.commands
            .iter()
            .any(|c| c == "pm disable-user --user 10 'org.demo'")
    );
    assert!(w.apply(&mut d, &p.id).await.is_err());
}
#[tokio::test]
async fn transport_loss_never_replays_the_mutation() {
    let dir = tempfile::tempdir().unwrap();
    let w = workspace(dir.path());
    let mut d = fake(vec![Err(PocError::new("connection", "gone"))]);
    let p = w
        .prepare(
            &mut d,
            &target(),
            Action::Shell {
                command: "echo hello".into(),
            },
        )
        .await
        .unwrap();
    let r = w.apply(&mut d, &p.id).await.unwrap();
    assert_eq!(r.status, OperationStatus::Unknown);
    assert!(w.apply(&mut d, &p.id).await.is_err());
    assert_eq!(d.commands.len(), 1);
}
#[tokio::test]
async fn active_home_cannot_be_disabled() {
    let dir = tempfile::tempdir().unwrap();
    let w = workspace(dir.path());
    let mut d = fake(vec![
        ok(ENABLED),
        ok("org.demo/.Main"),
        ok("org.demo/.Main\n"),
        ok("org.demo\n"),
    ]);
    assert!(
        w.prepare(
            &mut d,
            &target(),
            Action::Package {
                package: "org.demo".into(),
                action: PackageAction::Disable
            }
        )
        .await
        .is_err()
    );
}

#[tokio::test]
async fn recovery_rejects_state_changed_since_the_verified_operation() {
    let dir = tempfile::tempdir().unwrap();
    let w = workspace(dir.path());
    let mut d = fake(vec![
        ok(ENABLED),
        ok("org.home/.Main"),
        ok(ENABLED),
        ok("org.home/.Main"),
        ok("Success"),
        ok(DISABLED),
        ok(ENABLED),
    ]);
    let p = w
        .prepare(
            &mut d,
            &target(),
            Action::Package {
                package: "org.demo".into(),
                action: PackageAction::Disable,
            },
        )
        .await
        .unwrap();
    let row = w.apply(&mut d, &p.id).await.unwrap();
    let result = w.prepare_recovery(&mut d, &row.id).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind, "staleRecovery");
}

#[tokio::test]
async fn app_op_reviews_the_stored_package_mode_and_verifies_only_that_mode() {
    let dir = tempfile::tempdir().unwrap();
    let w = workspace(dir.path());
    let mut d = fake(vec![
        ok("SYSTEM_ALERT_WINDOW: ignore"),
        ok("SYSTEM_ALERT_WINDOW: ignore"),
        ok(""),
        ok("Uid mode: SYSTEM_ALERT_WINDOW: ignore\nSYSTEM_ALERT_WINDOW: allow"),
    ]);
    let p = w
        .prepare(
            &mut d,
            &target(),
            Action::AppOp {
                package: "org.demo".into(),
                op: "SYSTEM_ALERT_WINDOW".into(),
                mode: "allow".into(),
            },
        )
        .await
        .unwrap();
    assert!(p.effect.contains("not effective access"));
    let r = w.apply(&mut d, &p.id).await.unwrap();
    assert_eq!(r.status, OperationStatus::Verified);
    assert!(r.recovery.is_some());
    assert!(
        d.commands
            .iter()
            .any(|c| c == "cmd appops set --user 10 'org.demo' SYSTEM_ALERT_WINDOW allow")
    );
}
