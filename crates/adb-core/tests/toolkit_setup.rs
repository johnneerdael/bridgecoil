use adb_core::{
    backend::PocError,
    toolkit::{
        inventory::{Manager, ShizukuSlot, Stage, facts_command, inspect, parse_facts},
        setup::{SetupStep, deploy_command, plan, start_command},
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

/// Runs the real probe against a stand-in `ip`, with output in the format the TV image prints.
fn wired_fact(addresses: &str) -> bool {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let ip = dir.path().join("ip");
    std::fs::write(&ip, format!("#!/bin/sh\ncat <<'OUT'\n{addresses}\nOUT\n")).unwrap();
    std::fs::set_permissions(&ip, std::fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!(
        "{}:{}",
        dir.path().display(),
        std::env::var("PATH").unwrap()
    );
    let output = std::process::Command::new("sh")
        .args(["-c", facts_command()])
        .env("PATH", path)
        .output()
        .unwrap();
    parse_facts(&String::from_utf8_lossy(&output.stdout)).flag("wired")
}

#[test]
fn notices_a_tv_on_a_network_cable() {
    let lo = "1: lo    inet 127.0.0.1/8 scope host lo";
    let eth = "15: eth0    inet 10.0.2.15/24 brd 10.0.2.255 scope global eth0";
    let wlan = "16: wlan0    inet 192.168.1.20/24 brd 192.168.1.255 scope global wlan0";
    assert!(wired_fact(&format!("{lo}\n{eth}")));
    assert!(!wired_fact(&format!("{lo}\n{wlan}")));
    assert!(!wired_fact(&format!("{lo}\n{eth}\n{wlan}")));
    assert!(!wired_fact(lo));
    assert!(!wired_fact(""));
}

/// Runs the real step against stand-in Android tools and returns what the TV was told to do.
fn background_calls(manager: Manager) -> String {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("calls");
    for tool in ["am", "cmd", "dumpsys"] {
        let stub = dir.path().join(tool);
        let script = format!("#!/bin/sh\necho \"{tool} $*\" >> '{}'\n", log.display());
        std::fs::write(&stub, script).unwrap();
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let path = format!(
        "{}:{}",
        dir.path().display(),
        std::env::var("PATH").unwrap()
    );
    let command = plan(SetupStep::AllowBackground { manager }).command;
    let status = std::process::Command::new("sh")
        .args(["-c", &command])
        .env("PATH", path)
        .status()
        .unwrap();
    assert!(status.success());
    std::fs::read_to_string(log).unwrap()
}

#[test]
fn keeps_the_helper_app_running_as_well_as_termux() {
    let shizuku = background_calls(Manager::ShizukuNext);
    for package in ["moe.shizuku.privileged.api", "com.termux", "com.termux.boot"] {
        assert!(shizuku.contains(&format!("deviceidle whitelist +{package}\n")));
        assert!(shizuku.contains(&format!("set {package} RUN_IN_BACKGROUND allow\n")));
    }
    assert!(shizuku.contains(
        "am broadcast -n moe.shizuku.privileged.api/moe.shizuku.manager.receiver.WatchdogToggleReceiver -a moe.shizuku.privileged.api.WATCHDOG_ON\n"
    ));
    let porter = background_calls(Manager::Porter);
    assert!(porter.contains("deviceidle whitelist +eu.darken.porter\n"));
    assert!(!porter.contains("moe.shizuku.privileged.api"));
}

/// Runs the real probe against a stand-in `dumpsys`, `am` and `getprop` in the formats a real
/// Android 9 box printed.
fn old_tv_facts(start_on_boot: bool) -> String {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let tool = |name: &str, body: &str| {
        let path = dir.path().join(name);
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    };
    let boot = "        moe.shizuku.manager.receiver.BootCompleteReceiver";
    let other = "        moe.shizuku.manager.service.Other";
    let lists = if start_on_boot {
        format!("disabledComponents:\n{other}\nenabledComponents:\n{boot}")
    } else {
        format!("disabledComponents:\n{boot}\nenabledComponents:\n{other}")
    };
    tool(
        "dumpsys",
        &format!(
            "case \"$2\" in moe.shizuku.privileged.api) cat <<'OUT'\n{lists}\nOUT\n;; esac"
        ),
    );
    tool("getprop", "echo 28");
    tool("am", "echo 'Broadcast completed: result=0, data=\"1\"'");
    let path = format!(
        "{}:{}",
        dir.path().display(),
        std::env::var("PATH").unwrap()
    );
    let output = std::process::Command::new("sh")
        .args(["-c", facts_command()])
        .env("PATH", path)
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn reads_start_on_boot_and_the_watchdog_from_the_tv() {
    let on = parse_facts(&old_tv_facts(true));
    assert!(on.flag("shizukuBoot"));
    assert!(on.flag("watchdog"));
    assert!(on.flag("noWirelessDebugging"));
    assert!(!on.flag("porterBoot"));
    assert!(!parse_facts(&old_tv_facts(false)).flag("shizukuBoot"));
}

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
async fn an_unreadable_certificate_is_neither_trusted_nor_blamed_on_another_app() {
    let mut d = device(vec![(SHIZUKU, NEXT)], "");
    d.no_metadata = true;
    let snap = inspect(&mut d, &target()).await.unwrap();
    assert_eq!(snap.shizuku, ShizukuSlot::Unknown);
    assert_eq!(snap.manager, None);
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
