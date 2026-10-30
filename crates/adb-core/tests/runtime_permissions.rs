use adb_core::{
    backend::PocError,
    workspace::{
        driver::{ShellDriver, ShellOutput},
        engine::{Action, Workspace},
        history::History,
        metadata::{Metadata, Permission},
        types::*,
    },
};
use async_trait::async_trait;
struct Driver {
    target: Target,
    granted: bool,
    protection: u8,
    commands: Vec<String>,
}
#[async_trait]
impl ShellDriver for Driver {
    fn current_target(&self) -> Option<Target> {
        Some(self.target.clone())
    }
    async fn shell(&mut self, _: &Target, c: &str) -> Result<ShellOutput, PocError> {
        self.commands.push(c.into());
        self.granted = c.starts_with("pm grant");
        Ok(ShellOutput {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: Some(0),
        })
    }
    async fn metadata(
        &mut self,
        _: &Target,
        _: &[String],
    ) -> Result<Option<Vec<Metadata>>, PocError> {
        Ok(Some(vec![Metadata {
            package: "org.camera".into(),
            permissions: vec![Permission {
                name: "android.permission.CAMERA".into(),
                granted: self.granted,
                protection: Some(self.protection),
            }],
            ..Default::default()
        }]))
    }
}
fn driver() -> Driver {
    Driver {
        target: Target {
            device_id: "d".into(),
            session_id: "s".into(),
            profile_id: 10,
        },
        granted: false,
        protection: 1,
        commands: vec![],
    }
}
#[tokio::test]
async fn grant_is_profile_scoped_verified_and_recoverable() {
    let temp = tempfile::tempdir().unwrap();
    let w = Workspace::new(History::open(&temp.path().join("db")).unwrap());
    let mut d = driver();
    let t = d.target.clone();
    let p = w
        .prepare(
            &mut d,
            &t,
            Action::Permission {
                package: "org.camera".into(),
                permission: "android.permission.CAMERA".into(),
                grant: true,
            },
        )
        .await
        .unwrap();
    assert_eq!(d.commands.len(), 0);
    assert!(!p.losses.is_empty());
    let row = w.apply(&mut d, &p.id).await.unwrap();
    assert_eq!(row.status, OperationStatus::Verified);
    assert!(row.recovery.is_some());
    assert_eq!(
        d.commands[0],
        "pm grant --user 10 'org.camera' 'android.permission.CAMERA'"
    );
}
#[tokio::test]
async fn signature_permission_and_undeclared_permission_are_not_grantable() {
    let temp = tempfile::tempdir().unwrap();
    let w = Workspace::new(History::open(&temp.path().join("db")).unwrap());
    let mut d = driver();
    d.protection = 2;
    let t = d.target.clone();
    assert!(
        w.prepare(
            &mut d,
            &t,
            Action::Permission {
                package: "org.camera".into(),
                permission: "android.permission.CAMERA".into(),
                grant: true
            }
        )
        .await
        .is_err()
    );
    assert!(
        w.prepare(
            &mut d,
            &t,
            Action::Permission {
                package: "org.camera".into(),
                permission: "android.permission.RECORD_AUDIO".into(),
                grant: true
            }
        )
        .await
        .is_err()
    );
    assert!(d.commands.is_empty());
}
