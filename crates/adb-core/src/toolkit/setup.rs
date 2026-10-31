use super::inventory::{Facts, Manager};
use crate::shell_quote;
use serde::{Deserialize, Serialize};

const TERMUX_SHELL: &str = "/data/data/com.termux/files/home/.bridgecoil/shell";
const BOOT_SCRIPT: &str = "/data/data/com.termux/files/home/.termux/boot/00-bridgecoil";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SetupStep {
    StartServer { manager: Manager },
    OpenTerminal,
    AllowBackground,
    DeployShell { manager: Manager },
    RequestAccess,
    AllowBootStart { manager: Manager },
    InstallMonitor,
    RemoveMonitor,
}

fn with_args(args: &[&str], script: &str) -> String {
    let quoted: Vec<_> = args.iter().map(|a| shell_quote(a)).collect();
    format!("set -- {}\n{script}", quoted.join(" "))
}
pub fn start_command(manager: Manager) -> String {
    let (library, process) = match manager {
        Manager::ShizukuNext => ("libshizuku.so", "shizuku"),
        Manager::Porter => ("libporter.so", "porter"),
    };
    with_args(
        &[manager.package(), library, process],
        include_str!("scripts/start_server.sh"),
    )
}
pub fn deploy_command(manager: Manager) -> String {
    let (script, dex, id) = match manager {
        Manager::ShizukuNext => (
            "assets/rish",
            "assets/rish_shizuku.dex",
            "RISH_APPLICATION_ID",
        ),
        Manager::Porter => ("assets/porsh", "assets/porsh.dex", "PORSH_APPLICATION_ID"),
    };
    with_args(
        &[manager.package(), script, dex, id],
        include_str!("scripts/deploy_shell.sh"),
    )
}
pub fn verify_shell_command() -> &'static str {
    include_str!("scripts/verify_shell.sh")
}
const TERMINAL_OPEN: &str = "am start -n com.termux/.app.TermuxActivity\n\
i=0\n\
while [ $i -lt 8 ]; do\n\
  run-as com.termux test -x /data/data/com.termux/files/usr/bin/bash 2>/dev/null && exit 0\n\
  i=$((i + 1))\n\
  sleep 3\n\
done\n\
exit 0";
const BACKGROUND: &str = "for package in com.termux com.termux.boot; do\n\
  cmd appops set \"$package\" RUN_IN_BACKGROUND allow 2>/dev/null\n\
  cmd appops set \"$package\" RUN_ANY_IN_BACKGROUND allow 2>/dev/null\n\
  cmd appops set \"$package\" AUTO_START allow 2>/dev/null\n\
  dumpsys deviceidle whitelist \"+$package\" >/dev/null 2>&1\n\
done\n\
cmd appops write-settings 2>/dev/null\n\
am start -n com.termux.boot/.BootActivity >/dev/null 2>&1\n\
exit 0";

pub struct SetupPlan {
    pub title: String,
    pub effect: String,
    pub losses: Vec<String>,
    pub command: String,
}
pub fn plan(step: SetupStep) -> SetupPlan {
    let make = |title: String, effect: &str, losses: &[&str], command: String| SetupPlan {
        title,
        effect: effect.to_owned(),
        losses: losses.iter().map(|l| (*l).to_owned()).collect(),
        command,
    };
    match step {
        SetupStep::StartServer { manager } => make(
            format!("Start {}", manager.name()),
            "Start its background service over this ADB connection. It stops when the TV restarts.",
            &[],
            start_command(manager),
        ),
        SetupStep::OpenTerminal => make(
            "Prepare Termux".into(),
            "Open Termux on the TV once so it can finish setting itself up. This can take a minute.",
            &[],
            TERMINAL_OPEN.to_owned(),
        ),
        SetupStep::AllowBackground => make(
            "Let Termux run in the background".into(),
            "Allow Termux and Termux:Boot to start with the TV and keep running. Settings a TV doesn't have are skipped.",
            &["Background apps use a little memory, and some TVs ignore these settings."],
            BACKGROUND.to_owned(),
        ),
        SetupStep::DeployShell { manager } => make(
            format!("Connect Termux to {}", manager.name()),
            "Copy the shell tool from the installed manager into Termux.",
            &[],
            deploy_command(manager),
        ),
        SetupStep::RequestAccess => make(
            "Ask the TV for access".into(),
            "Show the permission prompt on the TV. Choose the option that always allows it.",
            &["Allowing this gives Termux the same power as ADB on this TV."],
            format!(
                "run-as com.termux sh -c {}",
                shell_quote(&format!(
                    "nohup timeout 120 {TERMUX_SHELL} -c id >/dev/null 2>&1 </dev/null &"
                ))
            ),
        ),
        SetupStep::AllowBootStart { manager } => make(
            format!("Let {} start with the TV", manager.name()),
            "Let the manager turn wireless debugging back on after a restart, then open it on the TV so you can switch on start at boot.",
            &["This permission lets the manager change secure system settings."],
            format!(
                "pm grant {} android.permission.WRITE_SECURE_SETTINGS || exit 1\n{}",
                shell_quote(manager.package()),
                with_args(
                    &[manager.package()],
                    include_str!("scripts/open_manager.sh")
                )
            ),
        ),
        SetupStep::InstallMonitor => make(
            "Keep mappings running".into(),
            "Add a Termux:Boot script that restarts your button mappings after a restart, retrying less often while the manager is unavailable.",
            &[
                "If the manager refuses three times, the script stops until the next restart, so it can't keep showing prompts on the TV.",
            ],
            format!(
                "{}{}MONITOR",
                include_str!("scripts/install_monitor.sh"),
                include_str!("scripts/boot_monitor.sh")
            ),
        ),
        SetupStep::RemoveMonitor => make(
            "Stop keeping mappings running".into(),
            "Remove only the script Bridgecoil added to Termux:Boot.",
            &["Mappings will stop after the next restart."],
            format!("run-as com.termux rm -f {}", shell_quote(BOOT_SCRIPT)),
        ),
    }
}
/// Steps whose effect can't be read back are reported as sent, never as verified.
pub fn has_readback(step: SetupStep) -> bool {
    !matches!(
        step,
        SetupStep::AllowBackground | SetupStep::RequestAccess | SetupStep::AllowBootStart { .. }
    )
}
pub fn verified(step: SetupStep, facts: &Facts) -> bool {
    match step {
        SetupStep::StartServer {
            manager: Manager::ShizukuNext,
        } => facts.flag("shizukuServer"),
        SetupStep::StartServer {
            manager: Manager::Porter,
        } => facts.flag("porterServer"),
        SetupStep::OpenTerminal => facts.flag("terminal"),
        SetupStep::DeployShell { .. } => facts.flag("shell"),
        SetupStep::InstallMonitor => facts.flag("monitor"),
        SetupStep::RemoveMonitor => !facts.flag("monitor"),
        _ => false,
    }
}
