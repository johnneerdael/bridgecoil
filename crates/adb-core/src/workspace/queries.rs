use super::metadata::Metadata;
use super::{commands::*, driver::*, engine::parse_package_state, types::*};
use crate::{backend::PocError, shell_quote};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap};
type Result<T> = std::result::Result<T, PocError>;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: u32,
    pub name: String,
    pub running: bool,
}
pub fn parse_profiles(raw: &str) -> Vec<Profile> {
    raw.lines()
        .filter_map(|line| {
            let body = line.split("UserInfo{").nth(1)?.split('}').next()?;
            let (id, rest) = body.split_once(':')?;
            let (name, _flags) = rest.rsplit_once(':')?;
            Some(Profile {
                id: id.parse().ok()?,
                name: name.into(),
                running: line.ends_with("running"),
            })
        })
        .collect()
}
pub fn parse_packages(raw: &str) -> Vec<String> {
    raw.lines()
        .filter_map(|l| l.trim().strip_prefix("package:"))
        .filter_map(|s| s.split_whitespace().next())
        .filter(|s| package_arg(s).is_ok())
        .map(str::to_owned)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
pub fn parse_components(raw: &str) -> Vec<String> {
    raw.lines()
        .map(str::trim)
        .filter(|s| component_arg(s).is_ok())
        .map(str::to_owned)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
#[derive(Debug, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Query {
    Device,
    Profiles,
    Packages {
        filter: String,
        search: String,
        page: usize,
        page_size: usize,
    },
    Package {
        package: String,
    },
    Launchers,
    Toolkit,
    Diagnostics {
        section: String,
    },
    Settings,
    Network,
    Files {
        path: String,
        page: usize,
        page_size: usize,
    },
}
async fn checked(d: &mut dyn ShellDriver, t: &Target, c: &str) -> Result<String> {
    validate_target(d.current_target().as_ref(), t)?;
    let o = d.shell(t, c).await?;
    if !o.success() {
        return Err(PocError::new(
            "deviceCommand",
            format!(
                "The TV could not complete this inspection: {}",
                o.combined()
            ),
        ));
    }
    Ok(o.stdout)
}
fn paged<T>(mut items: Vec<T>, page: usize, size: usize) -> Result<Page<T>> {
    if !(1..=32).contains(&size) || page > 1_000_000 {
        return Err(PocError::new("input", "Choose a page size from 1 to 32."));
    }
    let total = items.len();
    let start = page.saturating_mul(size).min(total);
    let end = start.saturating_add(size).min(total);
    let items = items.drain(start..end).collect();
    Ok(Page {
        items,
        total,
        page,
        page_size: size,
    })
}
pub async fn query(d: &mut dyn ShellDriver, t: &Target, q: Query) -> Result<Value> {
    validate_target(d.current_target().as_ref(), t)?;
    let user = t.profile_id;
    match q {
        Query::Profiles => Ok(json!(parse_profiles(
            &checked(d, t, "pm list users").await?
        ))),
        Query::Device => {
            let output=checked(d,t,"id; printf '\\n---STORAGE---\\n'; df -k /data; printf '\\n---BOOT---\\n'; cat /proc/sys/kernel/random/boot_id; printf '\\n---UPTIME---\\n'; cat /proc/uptime").await?;
            let identity = output.split("---STORAGE---").next().unwrap_or("").trim();
            let storage = output
                .split("---STORAGE---")
                .nth(1)
                .and_then(|s| s.split("---BOOT---").next())
                .unwrap_or("")
                .trim();
            let boot = output
                .split("---BOOT---")
                .nth(1)
                .and_then(|s| s.split("---UPTIME---").next())
                .unwrap_or("")
                .trim();
            let uptime = output
                .split("---UPTIME---")
                .nth(1)
                .unwrap_or("")
                .split_whitespace()
                .next()
                .unwrap_or("");
            Ok(
                json!({"shellIdentity":identity,"storage":storage,"bootId":boot,"uptimeSeconds":uptime,"observedMs":super::history::now_ms()}),
            )
        }
        Query::Packages {
            filter,
            search,
            page,
            page_size,
        } => {
            if search.len() > 255 {
                return Err(PocError::new("input", "Search text is too long."));
            }
            let flag = match filter.as_str() {
                "all" => "",
                "user" => "-3",
                "system" => "-s",
                "disabled" => "-d",
                "removed" => "-u",
                _ => return Err(PocError::new("input", "Choose an app filter.")),
            };
            let mut packages = parse_packages(
                &checked(d, t, &format!("pm list packages --user {user} {flag}")).await?,
            );
            if filter == "removed" {
                let installed = parse_packages(
                    &checked(d, t, &format!("pm list packages --user {user}")).await?,
                );
                packages.retain(|p| !installed.contains(p));
            }
            let mut label_note = None;
            let labels = match d.labels(t, &packages).await {
                Ok(Some(rows)) => rows,
                Ok(None) => Vec::new(),
                Err(e) => {
                    validate_target(d.current_target().as_ref(), t)?;
                    label_note = Some(e.message);
                    Vec::new()
                }
            };
            let labels: HashMap<_, _> =
                labels.into_iter().map(|m| (m.package.clone(), m)).collect();
            let label = |p: &String| {
                labels
                    .get(p)
                    .and_then(|m| m.label.as_deref())
                    .unwrap_or(p)
                    .to_lowercase()
            };
            let needle = search.to_lowercase();
            packages.retain(|p| p.to_lowercase().contains(&needle) || label(p).contains(&needle));
            packages.sort_by_cached_key(|p| (label(p), p.clone()));
            let ids = paged(packages, page, page_size)?;
            let details = match d.metadata(t, &ids.items).await {
                Ok(Some(rows)) => rows,
                Ok(None) => Vec::new(),
                Err(e) => {
                    validate_target(d.current_target().as_ref(), t)?;
                    label_note = Some(e.message);
                    Vec::new()
                }
            };
            let details: HashMap<_, _> = details
                .into_iter()
                .map(|m| (m.package.clone(), m))
                .collect();
            let items: Vec<_> = ids
                .items
                .into_iter()
                .map(|p| {
                    details
                        .get(&p)
                        .cloned()
                        .or_else(|| labels.get(&p).cloned())
                        .unwrap_or_else(|| Metadata {
                            package: p,
                            ..Default::default()
                        })
                })
                .collect();
            Ok(
                json!({"items":items,"total":ids.total,"page":ids.page,"pageSize":ids.page_size,"metadataNote":label_note.map(|_|"Some app details could not be read. Package IDs remain available.")}),
            )
        }

        Query::Package { package } => {
            let p = package_arg(&package)?;
            let raw = checked(d, t, &format!("dumpsys package {p}")).await?;
            let state = parse_package_state(&raw, user);
            let output=checked(d,t,&format!("cmd package query-activities --brief --user {user} -a android.intent.action.MAIN -c android.intent.category.LEANBACK_LAUNCHER -p {p}")).await;
            let mut activities = output.as_deref().map(parse_components).unwrap_or_default();
            let output=checked(d,t,&format!("cmd package query-activities --brief --user {user} -a android.intent.action.MAIN -c android.intent.category.LAUNCHER -p {p}")).await;
            activities.extend(output.as_deref().map(parse_components).unwrap_or_default());
            activities.sort();
            activities.dedup();
            let version_name = raw
                .lines()
                .find_map(|l| l.trim().strip_prefix("versionName="))
                .unwrap_or("");
            let permissions: Vec<_> = raw
                .lines()
                .map(str::trim)
                .filter(|l| l.contains("permission."))
                .take(200)
                .map(str::to_owned)
                .collect();
            let metadata = match d.metadata(t, std::slice::from_ref(&package)).await {
                Ok(Some(rows)) => rows.into_iter().next(),
                _ => {
                    validate_target(d.current_target().as_ref(), t)?;
                    None
                }
            };
            if let Some(m) = &metadata {
                activities.extend(m.activities.clone());
                activities.sort();
                activities.dedup();
            }
            Ok(
                json!({"package":package,"state":state,"versionName":version_name,"activities":activities,"permissions":permissions,"raw":raw,"metadata":metadata}),
            )
        }

        Query::Launchers => {
            let homes=checked(d,t,&format!("cmd package query-activities --brief --user {user} -a android.intent.action.MAIN -c android.intent.category.HOME")).await?;
            let current=checked(d,t,&format!("cmd package resolve-activity --brief --user {user} -a android.intent.action.MAIN -c android.intent.category.HOME")).await?;
            Ok(
                json!({"homes":parse_components(&homes),"current":parse_components(&current).first()}),
            )
        }
        Query::Toolkit => {
            let packages =
                parse_packages(&checked(d, t, &format!("pm list packages --user {user}")).await?);
            let tools:Vec<_>=[("Shizuku","moe.shizuku.privileged.api"),("Termux","com.termux"),("Termux:Boot","com.termux.boot")].into_iter().map(|(name,package)|json!({"name":name,"package":package,"installed":packages.iter().any(|p|p==package),"authorized":"unknown","running":"unknown","startup":"unknown"})).collect();
            Ok(json!(tools))
        }
        Query::Diagnostics { section } => {
            let command = match section.as_str() {
                "processes" => "ps -A",
                "memory" => "dumpsys meminfo",
                "services" => "dumpsys activity services",
                "logs" => "logcat -d -t 120 -v threadtime",
                "display" => "dumpsys display",
                "input" => "getevent -lp",
                "battery" => "dumpsys battery",
                _ => return Err(PocError::new("input", "Choose a diagnostics section.")),
            };
            Ok(
                json!({"section":section,"output":checked(d,t,command).await?,"observedMs":super::history::now_ms()}),
            )
        }
        Query::Settings => {
            let mut values = Vec::new();
            for (namespace, key, name) in [
                ("global", "window_animation_scale", "Window animation"),
                (
                    "global",
                    "transition_animation_scale",
                    "Transition animation",
                ),
                ("global", "animator_duration_scale", "Animator duration"),
                ("system", "screen_brightness", "Brightness"),
                ("system", "volume_music", "Media volume"),
            ] {
                let value = checked(
                    d,
                    t,
                    &format!("settings --user {user} get {namespace} {key}"),
                )
                .await;
                values.push(json!({"namespace":namespace,"key":key,"name":name,"value":value.as_deref().ok().map(str::trim),"error":value.err().map(|e|e.message)}));
            }
            Ok(json!(values))
        }
        Query::Network => Ok(
            json!({"output":checked(d,t,"ip address; ip route").await?,"forwarding":"Direct forwarding is not available in this build."}),
        ),
        Query::Files {
            path,
            page,
            page_size,
        } => {
            let path = remote_path(&path)?;
            // NUL separates paths, so whitespace/newlines in names cannot inject rows or commands.
            let raw = checked(
                d,
                t,
                &format!(
                    "find {} -maxdepth 1 -mindepth 1 -print0",
                    shell_quote(&path)
                ),
            )
            .await?;
            let mut entries: Vec<_> = raw
                .split('\0')
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect();
            entries.sort();
            Ok(json!({"path":path,"entries":paged(entries,page,page_size)?}))
        }
    }
}
