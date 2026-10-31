use super::catalog::{SHIZUKU_PACKAGE, ToolId, spec};
use crate::{
    backend::PocError,
    workspace::{
        commands::package_arg,
        driver::ShellDriver,
        queries::parse_packages,
        types::{Target, validate_target},
    },
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Manager {
    ShizukuNext,
    Porter,
    /// A Shizuku-type app that was already on the TV, which the owner chose to use. Bridgecoil
    /// finds its package on the TV each time, because no certificate vouches for it.
    Existing,
}
impl Manager {
    /// What Bridgecoil installs for this manager. It never installs an existing one.
    pub fn tool(self) -> Option<ToolId> {
        match self {
            Self::ShizukuNext => Some(ToolId::ShizukuNext),
            Self::Porter => Some(ToolId::Porter),
            Self::Existing => None,
        }
    }
    pub fn name(self) -> &'static str {
        self.tool().map_or("your helper app", |tool| spec(tool).name)
    }
    pub fn package(self) -> Option<&'static str> {
        self.tool().map(|tool| spec(tool).package)
    }
    fn server_fact(self) -> &'static str {
        match self {
            Self::ShizukuNext | Self::Existing => "shizukuServer",
            Self::Porter => "porterServer",
        }
    }
    fn boot_fact(self) -> &'static str {
        match self {
            Self::ShizukuNext => "shizukuBoot",
            Self::Porter => "porterBoot",
            Self::Existing => "helperBoot",
        }
    }
}

/// A Shizuku-type app found on the TV that Bridgecoil didn't install and can't vouch for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HelperApp {
    pub package: String,
    pub label: String,
}

/// What occupies the Shizuku package name, which several unrelated apps can claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ShizukuSlot {
    Empty,
    ShizukuNext,
    PorterCompat,
    Other,
    /// Installed, but its certificate couldn't be read this time.
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Stage {
    Manager,
    Server,
    Terminal,
    Shell,
    Access,
    Startup,
    Done,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatus {
    pub installed: bool,
    pub official: bool,
    pub version: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolkitSnapshot {
    pub shizuku: ShizukuSlot,
    pub shizuku_version: Option<String>,
    pub porter: ToolStatus,
    pub termux: ToolStatus,
    pub termux_boot: ToolStatus,
    pub manager: Option<Manager>,
    pub helper: Option<HelperApp>,
    pub server_running: bool,
    pub can_configure_terminal: bool,
    pub terminal_ready: bool,
    pub shell_ready: bool,
    pub authorized: bool,
    pub monitor: bool,
    pub listener: bool,
    pub wired: bool,
    pub no_wireless_debugging: bool,
    pub network_adb: bool,
    pub boot_start: bool,
    pub watchdog: bool,
    pub stage: Stage,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Facts(HashMap<String, String>);
impl Facts {
    pub fn flag(&self, key: &str) -> bool {
        self.0.get(key).is_some_and(|v| !v.is_empty())
    }
    pub fn value(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(String::as_str).filter(|v| !v.is_empty())
    }
    /// Whether each fact holds, without values such as process IDs that change between probes.
    pub fn summary(&self) -> String {
        let mut keys: Vec<_> = self.0.keys().filter(|k| self.flag(k)).cloned().collect();
        keys.sort();
        keys.iter().map(|k| format!("{k}=1\n")).collect()
    }
}
pub fn parse_facts(raw: &str) -> Facts {
    Facts(
        raw.lines()
            .filter_map(|line| line.trim().split_once(char::from(61)))
            .filter(|(key, _)| key.bytes().all(|b| b.is_ascii_alphanumeric()))
            .map(|(key, value)| (key.to_owned(), value.to_owned()))
            .collect(),
    )
}
pub fn facts_command() -> &'static str {
    include_str!("scripts/facts.sh")
}

pub async fn inspect(d: &mut dyn ShellDriver, t: &Target) -> Result<ToolkitSnapshot, PocError> {
    validate_target(d.current_target().as_ref(), t)?;
    let listed = d
        .shell(t, &format!("pm list packages --user {}", t.profile_id))
        .await?;
    let installed = parse_packages(&listed.stdout);
    let wanted: Vec<String> = [
        SHIZUKU_PACKAGE,
        "eu.darken.porter",
        "com.termux",
        "com.termux.boot",
    ]
    .into_iter()
    .filter(|p| installed.iter().any(|i| i == p))
    .map(str::to_owned)
    .collect();
    let metadata = match d.metadata(t, &wanted).await {
        Ok(Some(rows)) => Some(rows),
        _ => {
            validate_target(d.current_target().as_ref(), t)?;
            None
        }
    };
    let readable = metadata.is_some();
    let metadata = metadata.unwrap_or_default();
    let status = |package: &str, signer: &str| {
        let row = metadata.iter().find(|m| m.package == package);
        ToolStatus {
            installed: installed.iter().any(|i| i == package),
            official: row.is_some_and(|m| m.signers.iter().any(|s| s.eq_ignore_ascii_case(signer))),
            version: row.and_then(|m| m.version_name.clone()),
        }
    };
    let next = status(SHIZUKU_PACKAGE, spec(ToolId::ShizukuNext).signer_sha256);
    let compat = status(SHIZUKU_PACKAGE, spec(ToolId::PorterCompat).signer_sha256);
    let porter = status(
        spec(ToolId::Porter).package,
        spec(ToolId::Porter).signer_sha256,
    );
    let termux = status(
        spec(ToolId::Termux).package,
        spec(ToolId::Termux).signer_sha256,
    );
    let termux_boot = status(
        spec(ToolId::TermuxBoot).package,
        spec(ToolId::TermuxBoot).signer_sha256,
    );
    let shizuku = match (next.installed, next.official, compat.official) {
        (false, ..) => ShizukuSlot::Empty,
        (_, true, _) => ShizukuSlot::ShizukuNext,
        (_, _, true) => ShizukuSlot::PorterCompat,
        _ if readable => ShizukuSlot::Other,
        _ => ShizukuSlot::Unknown,
    };
    let output = d.shell(t, facts_command()).await?;
    let facts = parse_facts(&output.stdout);
    let found = facts.value("helper").filter(|p| package_arg(p).is_ok());
    let installed_manager = match (shizuku, porter.official) {
        (ShizukuSlot::ShizukuNext, true) if facts.flag("porterServer") => Some(Manager::Porter),
        (ShizukuSlot::ShizukuNext, _) => Some(Manager::ShizukuNext),
        (_, true) => Some(Manager::Porter),
        _ => None,
    };
    // A helper Bridgecoil can vouch for is simply that helper, so this is only for the others.
    let unverified = found.filter(|_| {
        !matches!(shizuku, ShizukuSlot::ShizukuNext | ShizukuSlot::PorterCompat)
            && installed_manager.is_none()
    });
    let manager = match unverified {
        Some(_) if facts.flag("useHelper") => Some(Manager::Existing),
        _ => installed_manager,
    };
    let helper = match unverified {
        Some(package) => {
            let wanted = [package.to_owned()];
            let label = match d.metadata(t, &wanted).await {
                Ok(Some(rows)) => rows.into_iter().find_map(|m| m.label),
                _ => None,
            };
            Some(HelperApp {
                package: package.to_owned(),
                label: label.unwrap_or_else(|| package.to_owned()),
            })
        }
        None => None,
    };
    let server_running = manager.is_some_and(|m| facts.flag(m.server_fact()));
    let terminal_ready =
        termux.official && termux_boot.official && facts.flag("runAs") && facts.flag("terminal");
    let shell_ready = facts.flag("shell");
    let authorized = shell_ready && facts.flag("authorized");
    let monitor = facts.flag("monitor");
    let boot_start = manager.is_some_and(|m| facts.flag(m.boot_fact()));
    let watchdog = facts.flag("watchdog");
    // Only Shizuku-Next has a watchdog to turn on.
    let comes_back = boot_start && (manager != Some(Manager::ShizukuNext) || watchdog);
    let stage = if manager.is_none() {
        Stage::Manager
    } else if !server_running {
        Stage::Server
    } else if !terminal_ready {
        Stage::Terminal
    } else if !shell_ready {
        Stage::Shell
    } else if !authorized {
        Stage::Access
    } else if !monitor || !comes_back {
        Stage::Startup
    } else {
        Stage::Done
    };
    Ok(ToolkitSnapshot {
        shizuku,
        shizuku_version: next.version.clone(),
        porter,
        termux,
        termux_boot,
        manager,
        helper,
        server_running,
        can_configure_terminal: facts.flag("runAs"),
        terminal_ready,
        shell_ready,
        authorized,
        monitor,
        listener: facts.flag("listener"),
        wired: facts.flag("wired"),
        no_wireless_debugging: facts.flag("noWirelessDebugging"),
        network_adb: facts.flag("networkAdb"),
        boot_start,
        watchdog,
        stage,
    })
}
