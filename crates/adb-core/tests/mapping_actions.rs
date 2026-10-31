use adb_core::{
    backend::PocError,
    remote::{
        capture::KeyIdentity,
        mappings::{Mapping, MappingChange},
    },
    workspace::{
        driver::{ShellDriver, ShellOutput},
        engine::{Action, Workspace},
        history::History,
        metadata::Metadata,
        queries::{Query, query},
        types::{OperationStatus, Target},
    },
};
use async_trait::async_trait;
use std::collections::VecDeque;

fn target() -> Target {
    Target {
        device_id: "d".into(),
        session_id: "s".into(),
        profile_id: 0,
    }
}
struct Fake {
    answers: VecDeque<String>,
    commands: Vec<String>,
}
#[async_trait]
impl ShellDriver for Fake {
    fn current_target(&self) -> Option<Target> {
        Some(target())
    }
    async fn labels(
        &mut self,
        _: &Target,
        wanted: &[String],
    ) -> Result<Option<Vec<Metadata>>, PocError> {
        Ok(Some(
            wanted
                .iter()
                .map(|p| Metadata {
                    package: p.clone(),
                    label: Some(format!("{p} label")),
                    ..Default::default()
                })
                .collect(),
        ))
    }
    async fn shell(&mut self, _: &Target, c: &str) -> Result<ShellOutput, PocError> {
        self.commands.push(c.into());
        Ok(ShellOutput {
            stdout: self.answers.pop_front().expect("unexpected command"),
            stderr: String::new(),
            exit_code: Some(0),
        })
    }
}
fn fake(answers: Vec<String>) -> Fake {
    Fake {
        answers: answers.into(),
        commands: vec![],
    }
}
fn workspace(dir: &std::path::Path) -> Workspace {
    Workspace::new(History::open(&dir.join("ops.db")).unwrap())
}
fn mapping() -> Mapping {
    Mapping {
        key: KeyIdentity {
            source: "Remote".into(),
            key: "KEY_UNKNOWN".into(),
            scan: Some("000c0057".into()),
        },
        component: "org.demo/.Main".into(),
        label: "Player".into(),
    }
}
fn state(running: bool, paused: bool, set: &[Mapping]) -> String {
    format!(
        "running={}\npaused={}\n---mappings\n{}\n---log\n",
        if running { "1" } else { "" },
        if paused { "1" } else { "" },
        if set.is_empty() {
            String::new()
        } else {
            serde_json::to_string(set).unwrap()
        }
    )
}
fn apply(set: Vec<Mapping>) -> Action {
    Action::Mappings {
        change: MappingChange::Apply { mappings: set },
    }
}

#[tokio::test]
async fn saving_says_what_the_button_will_still_do_and_verifies_the_tv() {
    let dir = tempfile::tempdir().unwrap();
    let w = workspace(dir.path());
    let set = vec![mapping()];
    let mut d = fake(vec![
        state(false, false, &[]),
        state(false, false, &[]),
        String::new(),
        state(true, false, &set),
    ]);
    let review = w.prepare(&mut d, &target(), apply(set)).await.unwrap();
    assert!(review.losses.iter().any(|l| l.contains("normal action")));
    assert!(review.reversible);
    let done = w.apply(&mut d, &review.id).await.unwrap();
    assert_eq!(done.status, OperationStatus::Verified);
    assert_eq!(done.kind, "mapping");
}

#[tokio::test]
async fn a_listener_that_did_not_start_is_not_reported_as_working() {
    let dir = tempfile::tempdir().unwrap();
    let w = workspace(dir.path());
    let set = vec![mapping()];
    let mut d = fake(vec![
        state(false, false, &[]),
        state(false, false, &[]),
        String::new(),
        state(false, false, &set),
    ]);
    let review = w.prepare(&mut d, &target(), apply(set)).await.unwrap();
    assert_eq!(
        w.apply(&mut d, &review.id).await.unwrap().status,
        OperationStatus::Unknown
    );
}

#[tokio::test]
async fn undoing_the_first_mappings_removes_them() {
    let dir = tempfile::tempdir().unwrap();
    let w = workspace(dir.path());
    let set = vec![mapping()];
    let after = state(true, false, &set);
    let mut d = fake(vec![
        state(false, false, &[]),
        state(false, false, &[]),
        String::new(),
        after.clone(),
        after.clone(),
        after,
    ]);
    let review = w.prepare(&mut d, &target(), apply(set)).await.unwrap();
    let done = w.apply(&mut d, &review.id).await.unwrap();
    let undo = w.prepare_recovery(&mut d, &done.id).await.unwrap();
    assert!(undo.commands[0].contains("rm -rf /data/local/tmp/bridgecoil"));
}

#[tokio::test]
async fn pausing_and_removing_are_verified() {
    let dir = tempfile::tempdir().unwrap();
    let w = workspace(dir.path());
    let set = vec![mapping()];
    let on = state(true, false, &set);
    let mut d = fake(vec![
        on.clone(),
        on.clone(),
        String::new(),
        state(true, true, &set),
    ]);
    let review = w
        .prepare(
            &mut d,
            &target(),
            Action::Mappings {
                change: MappingChange::Pause { paused: true },
            },
        )
        .await
        .unwrap();
    assert_eq!(
        w.apply(&mut d, &review.id).await.unwrap().status,
        OperationStatus::Verified
    );
    let mut d = fake(vec![
        on.clone(),
        on,
        String::new(),
        state(false, false, &[]),
    ]);
    let review = w
        .prepare(
            &mut d,
            &target(),
            Action::Mappings {
                change: MappingChange::RemoveAll,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        w.apply(&mut d, &review.id).await.unwrap().status,
        OperationStatus::Verified
    );
}

#[tokio::test]
async fn capture_reports_only_real_buttons() {
    let raw = "add device 1: /dev/input/event0\n  name:     \"Remote\"\n\
/dev/input/event0: EV_KEY       KEY_F1               DOWN\n";
    let result = query(
        &mut fake(vec![raw.into()]),
        &target(),
        Query::CaptureKeys { seconds: 8 },
    )
    .await
    .unwrap();
    assert_eq!(result["presses"][0]["key"], "KEY_F1");
    assert_eq!(result["presses"][0]["source"], "Remote");
}

#[tokio::test]
async fn silence_is_an_empty_capture_rather_than_an_error() {
    let result = query(
        &mut fake(vec![String::new()]),
        &target(),
        Query::CaptureKeys { seconds: 8 },
    )
    .await
    .unwrap();
    assert_eq!(result["presses"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn the_app_picker_lists_tv_and_phone_launchers_once_each() {
    let tv = "org.tv/.Main\norg.both/.Main\n";
    let phone = "org.phone/.Main\norg.both/.Main\n";
    let result = query(
        &mut fake(vec![tv.into(), phone.into()]),
        &target(),
        Query::LaunchableApps,
    )
    .await
    .unwrap();
    let apps = result.as_array().unwrap();
    assert_eq!(apps.len(), 3);
    assert_eq!(apps[0]["component"], "org.both/.Main");
    assert_eq!(apps[0]["label"], "org.both label");
}
