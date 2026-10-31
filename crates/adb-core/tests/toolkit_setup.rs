use adb_core::{
    backend::PocError,
    toolkit::{
        inventory::{Manager, ShizukuSlot, Stage, inspect, parse_facts},
        setup::{SetupStep, deploy_command, start_command},
    },
    workspace::{
        driver::{ShellDriver, ShellOutput},
        engine::{Action, Workspace},
        history::History,
        metadata::Metadata,
        types::{OperationStatus, Target},
    },
};
use async_trait::async_trait;

const NEXT: &str = "06360bb800d1a117c3a22c91a58e638bcd37281cb4d644fcfac5d7238c654293";
const PORTER: &str = "c6302c2b163c7f42f6d8bd658e53387880511020136a083f0a6a4bdc20b4bfa0";
const TERMUX: &str = "b6da01480eefd5fbf2cd3771b8d1021ec791304bdd6c4bf41d3faabad48ee5e1";

fn target() -> Target {
    Target {
        device_id: "d".into(),
        session_id: "s".into(),
        profile_id: 0,
    }
}
#[derive(Default)]
struct Fake {
    packages: Vec<(&'static str, &'static str)>,
    facts: String,
    commands: Vec<String>,
    no_metadata: bool,
}
#[async_trait]
impl ShellDriver for Fake {
    fn current_target(&self) -> Option<Target> {
        Some(target())
    }
    async fn metadata(
        &mut self,
        _: &Target,
        wanted: &[String],
    ) -> Result<Option<Vec<Metadata>>, PocError> {
        if self.no_metadata {
            return Ok(None);
        }
        Ok(Some(
            wanted
                .iter()
                .filter_map(|p| {
                    self.packages
                        .iter()
                        .find(|(n, _)| n == p)
                        .map(|(n, s)| Metadata {
                            package: (*n).into(),
                            version_name: Some("1.0".into()),
                            signers: vec![(*s).into()],
                            ..Default::default()
                        })
                })
                .collect(),
        ))
    }
    async fn shell(&mut self, _: &Target, c: &str) -> Result<ShellOutput, PocError> {
        self.commands.push(c.into());
        let stdout = if c.starts_with("pm list packages") {
            self.packages
                .iter()
                .map(|(n, _)| format!("package:{n}\n"))
                .collect()
        } else {
            self.facts.clone()
        };
        Ok(ShellOutput {
            stdout,
            stderr: String::new(),
            exit_code: Some(0),
        })
    }
}
fn device(packages: Vec<(&'static str, &'static str)>, facts: &str) -> Fake {
    Fake {
        packages,
        facts: facts.into(),
        ..Default::default()
    }
}
const SHIZUKU: &str = "moe.shizuku.privileged.api";

#[test]
fn reads_probe_facts_and_ignores_noise() {
    let facts = parse_facts("warning: x\nshizukuServer=123\nporterServer=\nterminal=1\n");
    assert!(facts.flag("shizukuServer"));
    assert!(!facts.flag("porterServer"));
    assert!(facts.flag("terminal"));
    assert!(!facts.flag("monitor"));
}

#[tokio::test]
async fn an_empty_tv_starts_at_the_manager_step() {
    let snap = inspect(&mut device(vec![], ""), &target()).await.unwrap();
    assert_eq!(snap.shizuku, ShizukuSlot::Empty);
    assert_eq!(snap.manager, None);
    assert_eq!(snap.stage, Stage::Manager);
}

#[tokio::test]
async fn a_stub_under_the_shizuku_name_is_not_called_shizuku_next() {
    let snap = inspect(&mut device(vec![(SHIZUKU, "abcd")], ""), &target())
        .await
        .unwrap();
    assert_eq!(snap.shizuku, ShizukuSlot::Other);
    assert_eq!(snap.manager, None);
    assert_eq!(snap.stage, Stage::Manager);
}

#[tokio::test]
async fn recognises_each_manager_by_certificate() {
    let next = inspect(&mut device(vec![(SHIZUKU, NEXT)], ""), &target())
        .await
        .unwrap();
    assert_eq!(next.shizuku, ShizukuSlot::ShizukuNext);
    assert_eq!(next.manager, Some(Manager::ShizukuNext));
    let compat = inspect(
        &mut device(vec![(SHIZUKU, PORTER), ("eu.darken.porter", PORTER)], ""),
        &target(),
    )
    .await
    .unwrap();
    assert_eq!(compat.shizuku, ShizukuSlot::PorterCompat);
    assert_eq!(compat.manager, Some(Manager::Porter));
}

#[tokio::test]
async fn an_unreadable_certificate_is_not_trusted() {
    let mut d = device(vec![(SHIZUKU, NEXT)], "");
    d.no_metadata = true;
    let snap = inspect(&mut d, &target()).await.unwrap();
    assert_eq!(snap.shizuku, ShizukuSlot::Other);
}

#[tokio::test]
async fn stages_follow_what_is_actually_proven() {
    let tools = || {
        vec![
            (SHIZUKU, NEXT),
            ("com.termux", TERMUX),
            ("com.termux.boot", TERMUX),
        ]
    };
    for (facts, stage) in [
        ("", Stage::Server),
        ("shizukuServer=1\n", Stage::Terminal),
        ("shizukuServer=1\nrunAs=1\nterminal=1\n", Stage::Shell),
        (
            "shizukuServer=1\nrunAs=1\nterminal=1\nshell=1\n",
            Stage::Access,
        ),
        (
            "shizukuServer=1\nrunAs=1\nterminal=1\nshell=1\nauthorized=1\n",
            Stage::Startup,
        ),
        (
            "shizukuServer=1\nrunAs=1\nterminal=1\nshell=1\nauthorized=1\nmonitor=1\n",
            Stage::Done,
        ),
    ] {
        let snap = inspect(&mut device(tools(), facts), &target())
            .await
            .unwrap();
        assert_eq!(snap.stage, stage, "{facts}");
    }
}

#[tokio::test]
async fn a_termux_from_another_source_is_flagged_rather_than_used() {
    let snap = inspect(
        &mut device(
            vec![
                (SHIZUKU, NEXT),
                ("com.termux", "abcd"),
                ("com.termux.boot", "abcd"),
            ],
            "shizukuServer=1\n",
        ),
        &target(),
    )
    .await
    .unwrap();
    assert!(snap.termux.installed);
    assert!(!snap.termux.official);
    assert_eq!(snap.stage, Stage::Terminal);
}

#[tokio::test]
async fn a_missing_boot_addon_holds_the_terminal_stage() {
    let snap = inspect(
        &mut device(
            vec![(SHIZUKU, NEXT), ("com.termux", TERMUX)],
            "shizukuServer=1\nrunAs=1\nterminal=1\n",
        ),
        &target(),
    )
    .await
    .unwrap();
    assert_eq!(snap.stage, Stage::Terminal);
}

#[test]
fn starting_a_server_works_on_any_cpu_layout() {
    let next = start_command(Manager::ShizukuNext);
    assert!(next.contains("moe.shizuku.privileged.api"));
    assert!(next.contains("libshizuku.so"));
    let porter = start_command(Manager::Porter);
    assert!(porter.contains("eu.darken.porter"));
    assert!(porter.contains("libporter.so"));
    assert!(
        !porter.contains("lib/arm64/"),
        "the CPU folder is discovered, not assumed"
    );
    // A starter kills every process mentioning its server, including a script that names it.
    assert!(!next.contains("shizuku_server"));
    assert!(!porter.contains("porter_server"));
}

#[test]
fn each_manager_gets_its_own_shell_wiring() {
    let next = deploy_command(Manager::ShizukuNext);
    assert!(next.contains("assets/rish_shizuku.dex"));
    assert!(next.contains("RISH_APPLICATION_ID"));
    let porter = deploy_command(Manager::Porter);
    assert!(porter.contains("assets/porsh.dex"));
    assert!(porter.contains("PORSH_APPLICATION_ID"));
}

#[tokio::test]
async fn a_started_server_is_verified_by_its_process() {
    let dir = tempfile::tempdir().unwrap();
    let w = Workspace::new(History::open(&dir.path().join("ops.db")).unwrap());
    let mut d = device(vec![], "shizukuServer=42\n");
    let review = w
        .prepare(
            &mut d,
            &target(),
            Action::Setup {
                step: SetupStep::StartServer {
                    manager: Manager::ShizukuNext,
                },
            },
        )
        .await
        .unwrap();
    assert!(review.title.contains("Shizuku-Next"));
    let done = w.apply(&mut d, &review.id).await.unwrap();
    assert_eq!(done.status, OperationStatus::Verified);
    assert_eq!(done.kind, "setup");
}

#[tokio::test]
async fn a_server_that_never_appears_is_not_verified() {
    let dir = tempfile::tempdir().unwrap();
    let w = Workspace::new(History::open(&dir.path().join("ops.db")).unwrap());
    let mut d = device(vec![], "");
    let review = w
        .prepare(
            &mut d,
            &target(),
            Action::Setup {
                step: SetupStep::StartServer {
                    manager: Manager::Porter,
                },
            },
        )
        .await
        .unwrap();
    assert_eq!(
        w.apply(&mut d, &review.id).await.unwrap().status,
        OperationStatus::Unknown
    );
}
