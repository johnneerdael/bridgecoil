use super::metadata::Metadata;
use crate::ratings::{
    RatingList,
    groups::{Group, group_for},
};
use super::{commands::*, driver::*, engine::parse_package_state, files, types::*};
use crate::{
    backend::PocError,
    remote::{
        capture::{capture_command, parse_capture},
        mappings,
    },
    toolkit::{inventory, setup},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
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
    AppOps {
        package: String,
    },
    Launchers,
    Toolkit,
    VerifyShell,
    CaptureKeys {
        seconds: u32,
    },
    Mappings,
    LaunchableApps,
    Screenshot,
    Diagnostics {
        section: String,
    },
    Settings,
    Network,
    Files {
        path: String,
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
/// Android lists Settings as a fallback Home. It must never look like a launcher to replace.
pub fn eligible_homes(components: Vec<String>) -> Vec<String> {
    components
        .into_iter()
        .filter(|c| !c.split('/').next().unwrap_or_default().ends_with(".settings"))
        .collect()
}
pub fn screenshot_command() -> &'static str {
    "screencap -p | base64"
}
const PNG_BASE64_START: &str = "iVBORw0KGgo";
pub fn parse_screenshot(raw: &str) -> Result<String> {
    let png: String = raw.split_whitespace().collect();
    if !png.starts_with(PNG_BASE64_START) {
        return Err(PocError::new(
            "captureUnavailable",
            "This TV didn't provide a screenshot.",
        ));
    }
    Ok(png)
}
pub async fn query(d: &mut dyn ShellDriver, t: &Target, q: Query) -> Result<Value> {
    query_rated(d, t, q, None).await
}
pub async fn query_rated(
    d: &mut dyn ShellDriver,
    t: &Target,
    q: Query,
    ratings: Option<&RatingList>,
) -> Result<Value> {
    validate_target(d.current_target().as_ref(), t)?;
    let user = t.profile_id;
    match q {
        Query::Profiles => Ok(json!(parse_profiles(
            &checked(d, t, "pm list users").await?
        ))),
        Query::AppOps { package } => {
            package_arg(&package)?;
            let mut rows = Vec::new();
            for (op, name) in super::app_ops::CATALOG {
                let command = super::app_ops::probe_command(user, &package, op)?;
                let result = checked(d, t, &command)
                    .await
                    .and_then(|raw| super::app_ops::parse(&raw, op));
                validate_target(d.current_target().as_ref(), t)?;
                match result {
                    Ok(mode) => rows.push(json!({"op":op,"name":name,"mode":mode,"error":null})),
                    Err(e) => rows.push(json!({"op":op,"name":name,"mode":null,"error":e.message})),
                }
            }
            Ok(json!(rows))
        }
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
            if labels.len() < packages.len() {
                label_note.get_or_insert("Some app labels were not returned.".into());
            }
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
            let user_installed: HashSet<String> = match ratings {
                None => HashSet::new(),
                Some(_) if filter == "user" => packages.iter().cloned().collect(),
                Some(_) => match checked(d, t, &format!(
                    "pm list packages --user {user} -3{}",
                    if filter == "removed" { " -u" } else { "" }
                ))
                .await
                {
                    Ok(raw) => parse_packages(&raw).into_iter().collect(),
                    Err(e) => {
                        validate_target(d.current_target().as_ref(), t)?;
                        label_note.get_or_insert(e.message);
                        HashSet::new()
                    }
                },
            };
            let group_of = |p: &String| {
                ratings.map(|list| group_for(list.get(p), user_installed.contains(p)))
            };
            packages.sort_by_cached_key(|p| (group_of(p), label(p), p.clone()));
            let mut group_counts = BTreeMap::<Group, usize>::new();
            for group in packages.iter().filter_map(group_of) {
                *group_counts.entry(group).or_default() += 1;
            }
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
                    let group = group_of(&p);
                    let rating = ratings.and_then(|list| list.get(&p));
                    let mut item = json!(
                        details
                            .get(&p)
                            .cloned()
                            .or_else(|| labels.get(&p).cloned())
                            .unwrap_or_else(|| Metadata {
                                package: p.clone(),
                                ..Default::default()
                            })
                    );
                    if let Some(group) = group {
                        item["group"] = json!(group);
                        item["rating"] = json!(rating);
                    }
                    item
                })
                .collect();
            let mut page = json!({"items":items,"total":ids.total,"page":ids.page,"pageSize":ids.page_size,"metadataNote":label_note.map(|_|"Some app details could not be read. Package IDs remain available.")});
            if ratings.is_some() {
                page["groups"] = group_counts
                    .into_iter()
                    .map(|(id, count)| json!({"id":id,"count":count}))
                    .collect();
            }
            Ok(page)
        }

        Query::Package { package } => {
            let p = package_arg(&package)?;
            let raw = checked(d, t, &format!("dumpsys package {p}")).await?;
            let state = parse_package_state(&raw, user);
            let output=checked(d,t,&format!("cmd package query-activities --brief --user {user} -a android.intent.action.MAIN -c android.intent.category.LEANBACK_LAUNCHER -p {p}")).await;
            let mut activities = output.as_deref().map(parse_components).unwrap_or_default();
            let mut launch_activity = activities.first().cloned();
            let output=checked(d,t,&format!("cmd package query-activities --brief --user {user} -a android.intent.action.MAIN -c android.intent.category.LAUNCHER -p {p}")).await;
            let phone_activities = output.as_deref().map(parse_components).unwrap_or_default();
            if launch_activity.is_none() {
                launch_activity = phone_activities.first().cloned();
            }
            activities.extend(phone_activities);
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
            if launch_activity.is_none() {
                launch_activity = activities.first().cloned();
            }
            let mut detail = json!({"package":package,"state":state,"versionName":version_name,"activities":activities,"launchActivity":launch_activity,"permissions":permissions,"raw":raw,"metadata":metadata});
            if let Some(list) = ratings {
                let rating = list.get(&package);
                let related: BTreeSet<_> = rating
                    .into_iter()
                    .flat_map(|r| r.dependencies.iter().chain(&r.needed_by))
                    .collect();
                let installed = if related.is_empty() {
                    Vec::new()
                } else {
                    checked(d, t, &format!("pm list packages --user {user}"))
                        .await
                        .map(|raw| parse_packages(&raw))
                        .unwrap_or_default()
                };
                detail["rating"] = json!(rating);
                detail["installedRelated"] = related
                    .into_iter()
                    .filter(|p| installed.contains(p))
                    .map(|p| json!(p))
                    .collect();
            }
            Ok(detail)
        }

        Query::Launchers => {
            let homes=checked(d,t,&format!("cmd package query-activities --brief --user {user} -a android.intent.action.MAIN -c android.intent.category.HOME")).await?;
            let current=checked(d,t,&format!("cmd package resolve-activity --brief --user {user} -a android.intent.action.MAIN -c android.intent.category.HOME")).await?;
            let system = parse_packages(
                &checked(d, t, &format!("pm list packages --user {user} -s")).await?,
            );
            let homes = eligible_homes(parse_components(&homes));
            let stock: Vec<_> = homes
                .iter()
                .filter(|c| c.split('/').next().is_some_and(|p| system.iter().any(|s| s == p)))
                .cloned()
                .collect();
            Ok(json!({"homes":homes,"stock":stock,"current":parse_components(&current).first()}))
        }
        Query::Screenshot => {
            let raw = d.shell_large(t, screenshot_command()).await?;
            Ok(json!({"png":parse_screenshot(&raw.stdout)?}))
        }
        Query::Toolkit => Ok(json!(inventory::inspect(d, t).await?)),
        Query::CaptureKeys { seconds } => {
            let raw = d.shell(t, &capture_command(seconds)?).await?;
            Ok(json!({"presses":parse_capture(&raw.stdout)}))
        }
        Query::Mappings => {
            let raw = checked(d, t, &mappings::state_command()).await?;
            Ok(json!(mappings::parse_state(&raw)))
        }
        Query::LaunchableApps => {
            let mut components = BTreeSet::new();
            for category in ["LEANBACK_LAUNCHER", "LAUNCHER"] {
                let raw = d
                    .shell(t, &format!(
                        "cmd package query-activities --brief --user {user} -a android.intent.action.MAIN -c android.intent.category.{category}"
                    ))
                    .await?;
                components.extend(parse_components(&raw.stdout));
            }
            let packages: Vec<_> = components
                .iter()
                .filter_map(|c| c.split('/').next())
                .map(str::to_owned)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            let labels = d.labels(t, &packages).await.ok().flatten().unwrap_or_default();
            let label = |package: &str| {
                labels
                    .iter()
                    .find(|m| m.package == package)
                    .and_then(|m| m.label.clone())
                    .unwrap_or_else(|| package.to_owned())
            };
            let mut seen = BTreeSet::new();
            let apps: Vec<_> = components
                .iter()
                .filter(|c| seen.insert(c.split('/').next().unwrap_or_default().to_owned()))
                .map(|c| json!({"component":c,"label":label(c.split('/').next().unwrap_or_default())}))
                .collect();
            Ok(json!(apps))
        }
        Query::VerifyShell => {
            let raw = checked(d, t, setup::verify_shell_command()).await?;
            let route = raw.lines().find_map(|l| l.trim().strip_prefix("route="));
            Ok(json!({"route":route.unwrap_or("none")}))
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
        Query::Files { path } => {
            let path = remote_path(&path)?;
            let raw = checked(d, t, &files::listing_command(&path)).await?;
            Ok(json!(files::parse_listing(&path, &raw)?))
        }
    }
}
