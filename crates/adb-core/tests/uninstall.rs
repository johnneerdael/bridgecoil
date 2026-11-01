//! Uninstalling deletes an app from every profile, so it is only for apps the owner added.
use adb_core::{
    backend::PocError,
    workspace::{
        driver::{ShellDriver, ShellOutput},
        engine::{Action, PackageAction, Workspace, is_system_app},
        history::History,
        types::{OperationStatus, Target},
    },
};
use async_trait::async_trait;

const INSTALLED: &str = "Packages:\n  Package [org.demo] (abc):\n    versionCode=3 minSdk=24\n    pkgFlags=[ HAS_CODE ALLOW_CLEAR_USER_DATA ]\n    User 0: ceDataInode=1 installed=true hidden=false suspended=false stopped=false enabled=0\n";
const SYSTEM: &str = "Packages:\n  Package [org.demo] (abc):\n    versionCode=3 minSdk=24\n    pkgFlags=[ SYSTEM HAS_CODE ALLOW_CLEAR_USER_DATA ]\n    User 0: ceDataInode=1 installed=true hidden=false suspended=false stopped=false enabled=0\n";
/// Android 9 answers for an unknown package with its general report.
const GONE_ANDROID_9: &str = "Package Changes:\n  Sequence number=34\n  User 0:\n    seq=16, package=com.other\n";
const GONE_ANDROID_14: &str = "Unable to find package: org.demo\n";

struct Tv {
    report: &'static str,
    after: &'static str,
    sent: Vec<String>,
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
    async fn shell(&mut self, _: &Target, c: &str) -> Result<ShellOutput, PocError> {
        self.sent.push(c.into());
        let uninstalled = self.sent.iter().any(|s| s.starts_with("pm uninstall"));
        let stdout = if c.starts_with("pm uninstall") {
            "Success\n"
        } else if c.starts_with("dumpsys package") {
            if uninstalled { self.after } else { self.report }
        } else {
            "priority=0 preferredOrder=0\ncom.vendor.home/.Main\n"
        };
        Ok(ShellOutput {
            stdout: stdout.into(),
            stderr: String::new(),
            exit_code: Some(0),
        })
    }
}
fn workspace(dir: &tempfile::TempDir) -> Workspace {
    Workspace::new(History::open(&dir.path().join("ops.db")).unwrap())
}
fn uninstall() -> Action {
    Action::Package {
        package: "org.demo".into(),
        action: PackageAction::Uninstall,
    }
}

#[test]
fn only_an_app_marked_system_counts_as_system() {
    assert!(is_system_app(SYSTEM));
    assert!(!is_system_app(INSTALLED));
    assert!(!is_system_app("pkgFlags=[ HAS_CODE NOT_SYSTEM ]"));
}

#[tokio::test]
async fn an_uninstall_removes_the_app_for_every_profile_and_is_verified_gone() {
    for after in [GONE_ANDROID_9, GONE_ANDROID_14] {
        let dir = tempfile::tempdir().unwrap();
        let w = workspace(&dir);
        let mut tv = Tv {
            report: INSTALLED,
            after,
            sent: vec![],
        };
        let review = w.prepare(&mut tv, &target(), uninstall()).await.unwrap();
        assert_eq!(review.commands, ["pm uninstall 'org.demo'"]);
        assert!(!review.reversible);
        let result = w.apply(&mut tv, &review.id).await.unwrap();
        assert!(matches!(result.status, OperationStatus::Verified), "{after}");
    }
}

#[tokio::test]
async fn an_app_that_is_still_there_is_not_reported_as_uninstalled() {
    let dir = tempfile::tempdir().unwrap();
    let w = workspace(&dir);
    let mut tv = Tv {
        report: INSTALLED,
        after: INSTALLED,
        sent: vec![],
    };
    let review = w.prepare(&mut tv, &target(), uninstall()).await.unwrap();
    let result = w.apply(&mut tv, &review.id).await.unwrap();
    assert!(!matches!(result.status, OperationStatus::Verified));
}

#[tokio::test]
async fn a_system_app_is_refused_before_anything_is_sent() {
    let dir = tempfile::tempdir().unwrap();
    let w = workspace(&dir);
    let mut tv = Tv {
        report: SYSTEM,
        after: SYSTEM,
        sent: vec![],
    };
    let error = w.prepare(&mut tv, &target(), uninstall()).await.unwrap_err();
    assert_eq!(error.kind, "protected");
    assert!(!tv.sent.iter().any(|c| c.starts_with("pm uninstall")));
}
