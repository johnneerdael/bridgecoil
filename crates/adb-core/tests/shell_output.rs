//! A command someone runs in Shell is cut off when it says too much. Bridgecoil's own reads still
//! refuse, because a parser fed half a report would quietly get the wrong answer.
use adb_core::{
    backend::PocError,
    workspace::{
        driver::{ShellDriver, ShellOutput},
        engine::{Action, Workspace},
        history::History,
        types::{OperationStatus, Target},
    },
};
use async_trait::async_trait;

struct Tv {
    code: Option<u8>,
}
fn target() -> Target {
    Target {
        device_id: "d".into(),
        session_id: "s".into(),
        profile_id: 0,
    }
}
#[async_trait]
impl ShellDriver for Tv {
    fn current_target(&self) -> Option<Target> {
        Some(target())
    }
    async fn shell(&mut self, _: &Target, _: &str) -> Result<ShellOutput, PocError> {
        Err(PocError::new("outputLimit", "Refused"))
    }
    async fn shell_for_person(&mut self, _: &Target, _: &str) -> Result<ShellOutput, PocError> {
        Ok(ShellOutput {
            stdout: "first part\n".into(),
            stderr: String::new(),
            exit_code: self.code,
        })
    }
}
async fn run(code: Option<u8>) -> adb_core::workspace::types::OperationRecord {
    let dir = tempfile::tempdir().unwrap();
    let w = Workspace::new(History::open(&dir.path().join("ops.db")).unwrap());
    let mut tv = Tv { code };
    let action = Action::Shell {
        command: "logcat -d".into(),
    };
    let review = w.prepare(&mut tv, &target(), action).await.unwrap();
    w.apply(&mut tv, &review.id).await.unwrap()
}

#[tokio::test]
async fn shell_commands_take_the_route_that_keeps_the_connection() {
    let result = run(Some(0)).await;
    assert!(matches!(result.status, OperationStatus::Sent));
    assert!(result.output.contains("first part"));
}

#[tokio::test]
async fn a_failed_command_says_its_exit_code_in_words() {
    let result = run(Some(2)).await;
    assert!(matches!(result.status, OperationStatus::Failed));
    assert_eq!(result.error.as_deref(), Some("The TV reported exit code 2."));
    assert!(result.output.contains("first part"));
}
