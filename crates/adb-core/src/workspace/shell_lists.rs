//! The live lists behind the Shell's Normal mode, so a dropdown shows what's on this TV right now.
//! Each source is one fixed read-only command, and nothing here runs text it was handed.
use super::commands::package_arg;
use crate::backend::PocError;
use serde::Serialize;
use std::collections::BTreeSet;

type Result<T> = std::result::Result<T, PocError>;
const LIMIT: usize = 2000;
const TERMUX_HOME: &str = "/data/data/com.termux/files/home";
const TERMUX_PREFIX: &str = "/data/data/com.termux/files/usr";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ListItem {
    pub value: String,
    pub about: String,
}
fn item(value: impl Into<String>, about: impl Into<String>) -> ListItem {
    ListItem {
        value: value.into(),
        about: about.into(),
    }
}
fn plain(value: impl Into<String>) -> ListItem {
    item(value, "")
}

fn package_of(argument: Option<&str>) -> Result<String> {
    package_arg(argument.unwrap_or_default())
}

pub fn list_command(source: &str, argument: Option<&str>, user: u32) -> Result<String> {
    Ok(match source {
        "commands" => "ls /system/bin /system/xbin /vendor/bin /vendor/xbin /product/bin 2>/dev/null; run-as com.termux true 2>/dev/null && echo termux".into(),
        "packages" => format!("pm list packages --user {user}"),
        "disabledPackages" => format!("pm list packages --user {user} -d"),
        "userPackages" => format!("pm list packages --user {user} -3"),
        "permissions" | "activities" => format!("dumpsys package {}", package_of(argument)?),
        "settings" => match argument {
            Some(space @ ("system" | "secure" | "global")) => format!("settings list {space}"),
            _ => return Err(PocError::new("input", "Choose a settings group.")),
        },
        "properties" => "getprop".into(),
        "services" => "dumpsys -l".into(),
        "cmdServices" => "cmd -l".into(),
        "users" => "pm list users".into(),
        "processes" => "ps -A -o PID=,NAME=".into(),
        "shellProcesses" => "ps -A -o PID=,USER=,NAME=".into(),
        "appOps" => format!("cmd appops get {}", package_of(argument)?),
        "interfaces" => "ls /sys/class/net".into(),
        "keyboards" => "ime list -s".into(),
        "features" => "pm list features".into(),
        "termuxPackages" => format!(
            "run-as com.termux {TERMUX_PREFIX}/bin/env -i HOME={TERMUX_HOME} PREFIX={TERMUX_PREFIX} PATH={TERMUX_PREFIX}/bin TMPDIR={TERMUX_PREFIX}/tmp {TERMUX_PREFIX}/bin/apt list --installed"
        ),
        "termuxFiles" => format!(
            "run-as com.termux find {TERMUX_HOME} -maxdepth 3 -type f -not -path '*/.bridgecoil/*' -not -path '*/.ssh/*'"
        ),
        _ => return Err(PocError::new("input", "Unknown list.")),
    })
}

/// The permissions an app asks for, and whether it has each one.
fn permissions(raw: &str) -> Vec<ListItem> {
    let mut requested = BTreeSet::new();
    let mut granted = BTreeSet::new();
    let mut in_requested = false;
    for line in raw.lines() {
        let text = line.trim();
        if text == "requested permissions:" {
            in_requested = true;
            continue;
        }
        if in_requested {
            let name = text.split(':').next().unwrap_or_default();
            if name.contains('.') && !name.contains(' ') {
                requested.insert(name.to_owned());
                continue;
            }
            in_requested = false;
        }
        if let Some((name, state)) = text.split_once(": granted=")
            && state.starts_with("true")
        {
            granted.insert(name.to_owned());
        }
    }
    requested
        .into_iter()
        .map(|name| {
            let about = if granted.contains(&name) {
                "Granted"
            } else {
                "Not granted"
            };
            item(name, about)
        })
        .collect()
}

/// Screens that can be opened from outside, which are the ones Android lists by intent.
fn activities(raw: &str, package: &str) -> Vec<ListItem> {
    let needle = format!("{package}/");
    let mut found = BTreeSet::new();
    let mut in_table = false;
    for line in raw.lines() {
        if line.ends_with("Resolver Table:") {
            in_table = line.starts_with("Activity Resolver Table");
            continue;
        }
        if !in_table {
            continue;
        }
        if let Some(component) = line
            .split_whitespace()
            .find(|word| word.starts_with(&needle))
        {
            found.insert(component.to_owned());
        }
    }
    found.into_iter().map(plain).collect()
}

pub fn parse_list(source: &str, argument: Option<&str>, raw: &str) -> Vec<ListItem> {
    let lines = raw.lines().map(str::trim).filter(|l| !l.is_empty());
    let mut items: Vec<ListItem> = match source {
        "commands" => raw
            .split_whitespace()
            .filter(|word| !word.ends_with(':'))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(plain)
            .collect(),
        "packages" | "disabledPackages" | "userPackages" => super::queries::parse_packages(raw)
            .into_iter()
            .map(plain)
            .collect(),
        "permissions" => permissions(raw),
        "activities" => activities(raw, argument.unwrap_or_default()),
        "settings" => lines
            .filter_map(|line| line.split_once('='))
            .map(|(key, value)| item(key, value.chars().take(60).collect::<String>()))
            .collect(),
        "properties" => lines
            .filter_map(|line| line.strip_prefix('['))
            .filter_map(|line| line.split_once("]: ["))
            .map(|(key, value)| {
                let value = value.trim_end_matches(']');
                item(key, value.chars().take(60).collect::<String>())
            })
            .collect(),
        "services" | "cmdServices" => lines
            .filter(|line| !line.ends_with(':'))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(plain)
            .collect(),
        "users" => lines
            .filter_map(|line| line.strip_prefix("UserInfo{"))
            .filter_map(|line| {
                let mut parts = line.split(':');
                Some(item(parts.next()?, parts.next()?))
            })
            .collect(),
        "processes" => lines
            .filter_map(|line| line.split_once(char::is_whitespace))
            .map(|(pid, name)| item(pid, name.trim()))
            .collect(),
        "shellProcesses" => lines
            .filter_map(|line| {
                let mut parts = line.split_whitespace();
                let (pid, user, name) = (parts.next()?, parts.next()?, parts.next()?);
                (user == "shell").then(|| item(pid, name))
            })
            .collect(),
        "appOps" => lines
            .filter_map(|line| line.strip_prefix("Uid mode: ").or(Some(line)))
            .filter_map(|line| line.split_once(": "))
            .filter(|(name, _)| name.chars().all(|c| c.is_ascii_uppercase() || c == '_'))
            .map(|(name, mode)| item(name, mode.split(';').next().unwrap_or_default()))
            .collect(),
        "interfaces" | "keyboards" => lines.map(plain).collect(),
        "features" => lines
            .filter_map(|line| line.strip_prefix("feature:"))
            .map(plain)
            .collect(),
        "termuxPackages" => lines
            .filter_map(|line| line.split_once('/'))
            .map(|(name, rest)| item(name, rest.split_whitespace().nth(1).unwrap_or_default()))
            .collect(),
        "termuxFiles" => lines
            .filter(|line| line.starts_with('/'))
            .map(plain)
            .collect(),
        _ => Vec::new(),
    };
    items.truncate(LIMIT);
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(items: Vec<ListItem>) -> Vec<(String, String)> {
        items.into_iter().map(|i| (i.value, i.about)).collect()
    }

    #[test]
    fn every_list_is_a_fixed_read_only_command() {
        for source in [
            "commands", "packages", "disabledPackages", "userPackages", "properties", "services",
            "cmdServices", "users", "processes", "shellProcesses", "interfaces", "keyboards",
            "features", "termuxPackages", "termuxFiles",
        ] {
            let command = list_command(source, None, 0).unwrap();
            for word in ["rm ", "disable", "uninstall", "put ", "set"] {
                assert!(!command.contains(word), "{source}: {command}");
            }
        }
    }

    #[test]
    fn text_from_the_page_never_reaches_a_command_unchecked() {
        assert!(list_command("permissions", Some("a;reboot"), 0).is_err());
        assert!(list_command("appOps", Some("$(reboot)"), 0).is_err());
        assert!(list_command("settings", Some("system;reboot"), 0).is_err());
        assert!(list_command("nothing", None, 0).is_err());
    }

    #[test]
    fn a_profile_only_lists_its_own_apps() {
        assert_eq!(
            list_command("userPackages", None, 10).unwrap(),
            "pm list packages --user 10 -3"
        );
    }

    #[test]
    fn permissions_say_whether_the_app_has_them() {
        let raw = "    requested permissions:\n      android.permission.INTERNET\n      android.permission.CAMERA: restricted=true\n    install permissions:\n      android.permission.INTERNET: granted=true\n    User 0: ceDataInode=1\n      runtime permissions:\n        android.permission.CAMERA: granted=false, flags=[ ]\n";
        assert_eq!(
            pairs(parse_list("permissions", Some("org.demo"), raw)),
            [
                ("android.permission.CAMERA".into(), "Not granted".into()),
                ("android.permission.INTERNET".into(), "Granted".into()),
            ]
        );
    }

    #[test]
    fn activities_come_only_from_the_activity_table() {
        let raw = "Activity Resolver Table:\n  Non-Data Actions:\n      android.intent.action.MAIN:\n        5d2 org.demo/.Main filter 7a\n        8c1 org.other/.Main filter 7b\nService Resolver Table:\n      a.b:\n        1aa org.demo/.Work filter 1\n";
        assert_eq!(
            pairs(parse_list("activities", Some("org.demo"), raw)),
            [("org.demo/.Main".into(), "".into())]
        );
    }

    #[test]
    fn settings_and_properties_show_their_current_value() {
        assert_eq!(
            pairs(parse_list("settings", None, "adb_enabled=1\nname=a=b\n")),
            [
                ("adb_enabled".into(), "1".into()),
                ("name".into(), "a=b".into()),
            ]
        );
        assert_eq!(
            pairs(parse_list("properties", None, "[ro.a]: [1]\n[ro.b]: []\n")),
            [("ro.a".into(), "1".into()), ("ro.b".into(), "".into())]
        );
    }

    #[test]
    fn services_drop_the_heading_android_puts_first() {
        assert_eq!(
            pairs(parse_list("services", None, "Currently running services:\n  SurfaceFlinger\n  wifi\n")),
            [("SurfaceFlinger".into(), "".into()), ("wifi".into(), "".into())]
        );
    }

    #[test]
    fn profiles_come_from_their_user_info_line() {
        assert_eq!(
            pairs(parse_list("users", None, "Users:\n\tUserInfo{0:Owner:13} running\n\tUserInfo{10:new_user:1010}\n")),
            [("0".into(), "Owner".into()), ("10".into(), "new_user".into())]
        );
    }

    #[test]
    fn only_the_shell_s_own_programs_can_be_ended() {
        assert_eq!(
            pairs(parse_list("shellProcesses", None, " 293 shell sh\n 2978 u0_a53 com.google.mediashell\n")),
            [("293".into(), "sh".into())]
        );
    }

    #[test]
    fn app_ops_cover_both_android_layouts() {
        assert_eq!(
            pairs(parse_list("appOps", None, "Uid mode: READ_EXTERNAL_STORAGE: ignore\nTOAST_WINDOW: allow; time=+20h ago\n")),
            [
                ("READ_EXTERNAL_STORAGE".into(), "ignore".into()),
                ("TOAST_WINDOW".into(), "allow".into()),
            ]
        );
    }

    #[test]
    fn program_names_drop_folder_headings() {
        let names = pairs(parse_list("commands", None, "/system/bin:\nls\nping\nbase64\n\n/vendor/bin:\nls\ntermux\n"));
        assert!(names.iter().any(|(n, _)| n == "termux"));
        assert!(names.iter().any(|(n, _)| n == "base64"));
        assert!(!names.iter().any(|(n, _)| n.ends_with(':')));
        assert_eq!(names.iter().filter(|(n, _)| n == "ls").count(), 1);
    }

    #[test]
    fn termux_packages_show_their_version() {
        assert_eq!(
            pairs(parse_list("termuxPackages", None, "Listing...\napt/stable,now 2.8.1 aarch64 [installed]\n")),
            [("apt".into(), "2.8.1".into())]
        );
    }
}
