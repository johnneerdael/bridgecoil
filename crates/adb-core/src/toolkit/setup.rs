use super::inventory::{Facts, Manager};
use crate::shell_quote;
use serde::{Deserialize, Serialize};

const TERMUX_SHELL: &str = "/data/data/com.termux/files/home/.bridgecoil/shell";
const BOOT_SCRIPT: &str = "/data/data/com.termux/files/home/.termux/boot/00-bridgecoil";
const USE_HELPER: &str = "/data/local/tmp/bridgecoil-use-helper";
/// Finds an existing helper on the TV, so nothing has to name it in advance.
const FIND_HELPER: &str = "helper=$(cmd package query-activities --brief \
-a rikka.shizuku.intent.action.REQUEST_BINDER 2>/dev/null | \
sed -n 's/^ *\\([A-Za-z0-9_.]*\\)\\/.*/\\1/p' | head -n 1)\n\
[ -n \"$helper\" ] || { echo \"The helper app wasn't found.\" >&2; exit 1; }\n";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SetupStep {
    StartServer { manager: Manager },
    OpenTerminal,
    AllowBackground { manager: Manager },
    DeployShell { manager: Manager },
    RequestAccess,
    AllowBootStart { manager: Manager },
    InstallMonitor,
    RemoveMonitor,
    UseHelper,
    StopUsingHelper,
}

fn with_args(args: &[&str], script: &str) -> String {
    let quoted: Vec<_> = args.iter().map(|a| shell_quote(a)).collect();
    format!("set -- {}\n{script}", quoted.join(" "))
}
/// Sets "$1" to the manager's package and the rest to `args`, then runs `script`.
fn for_manager(manager: Manager, args: &[&str], script: &str) -> String {
    match manager.package() {
        Some(package) => with_args(&[&[package], args].concat(), script),
        None => {
            let quoted: Vec<_> = args.iter().map(|a| shell_quote(a)).collect();
            format!("{FIND_HELPER}set -- \"$helper\" {}\n{script}", quoted.join(" "))
        }
    }
}
pub fn start_command(manager: Manager) -> String {
    let (library, process) = match manager {
        Manager::ShizukuNext | Manager::Existing => ("libshizuku.so", "shizuku"),
        Manager::Porter => ("libporter.so", "porter"),
    };
    for_manager(
        manager,
        &[library, process],
        include_str!("scripts/start_server.sh"),
    )
}
pub fn deploy_command(manager: Manager) -> String {
    let (script, dex, id) = match manager {
        Manager::ShizukuNext | Manager::Existing => (
            "assets/rish",
            "assets/rish_shizuku.dex",
            "RISH_APPLICATION_ID",
        ),
        Manager::Porter => ("assets/porsh", "assets/porsh.dex", "PORSH_APPLICATION_ID"),
    };
    for_manager(
        manager,
        &[script, dex, id],
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
const BACKGROUND: &str = "for package in \"$1\" com.termux com.termux.boot; do\n\
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
        SetupStep::AllowBackground { manager } => {
            let (watchdog, command) = match manager {
                Manager::ShizukuNext => {
                    let package = manager.package().unwrap_or_default();
                    (
                        " It also turns on Shizuku-Next's watchdog, which restarts its service if it stops.",
                        format!(
                            "am broadcast -n {package}/moe.shizuku.manager.receiver.WatchdogToggleReceiver -a {package}.WATCHDOG_ON >/dev/null 2>&1\n"
                        ),
                    )
                }
                Manager::Porter | Manager::Existing => ("", String::new()),
            };
            make(
                format!("Let Termux and {} keep running", manager.name()),
                &format!(
                    "Allow {}, Termux and Termux:Boot to start with the TV and keep running, without being stopped to save power. Settings a TV doesn't have are skipped.{watchdog}",
                    manager.name()
                ),
                &["Background apps use a little memory, and some TVs ignore these settings."],
                format!("{command}{}", for_manager(manager, &[], BACKGROUND)),
            )
        }
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
            "Give the manager the permission it needs to start with the TV, then open it on the TV so you can turn on Start on boot.",
            &["This permission lets the manager change secure system settings."],
            for_manager(
                manager,
                &[],
                concat!(
                    "pm grant \"$1\" android.permission.WRITE_SECURE_SETTINGS || exit 1\n",
                    include_str!("scripts/open_manager.sh")
                ),
            ),
        ),
        SetupStep::UseHelper => make(
            "Use the helper already on this TV".into(),
            "Bridgecoil will use the Shizuku-type app already on this TV instead of installing one. Termux gets that app's shell tool.",
            &["Bridgecoil hasn't verified this build. Your button mappings will run through it, so only continue if you trust where it came from."],
            format!("touch {USE_HELPER}"),
        ),
        SetupStep::StopUsingHelper => make(
            "Stop using this helper".into(),
            "Bridgecoil stops using the helper that was already on this TV. The app itself stays installed.",
            &["Mappings won't come back after a restart until setup is finished with another helper."],
            format!("rm -f {USE_HELPER}"),
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
        SetupStep::AllowBackground { .. }
            | SetupStep::RequestAccess
            | SetupStep::AllowBootStart { .. }
    )
}
pub fn verified(step: SetupStep, facts: &Facts) -> bool {
    match step {
        SetupStep::StartServer {
            manager: Manager::ShizukuNext | Manager::Existing,
        } => facts.flag("shizukuServer"),
        SetupStep::StartServer {
            manager: Manager::Porter,
        } => facts.flag("porterServer"),
        SetupStep::OpenTerminal => facts.flag("terminal"),
        SetupStep::DeployShell { .. } => facts.flag("shell"),
        SetupStep::InstallMonitor => facts.flag("monitor"),
        SetupStep::RemoveMonitor => !facts.flag("monitor"),
        SetupStep::UseHelper => facts.flag("useHelper"),
        SetupStep::StopUsingHelper => !facts.flag("useHelper"),
        _ => false,
    }
}
