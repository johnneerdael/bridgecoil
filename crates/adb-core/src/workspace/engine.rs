//! Single-use, target-bound reviews. Only this module turns a review into a mutation.
use super::{
    files,
    commands::*,
    driver::*,
    history::{History, now_ms},
    types::*,
};
use crate::{
    backend::PocError,
    ratings::{Level, store::Ratings},
    remote::mappings::{self, MappingChange, MappingSnapshot},
    shell_quote,
    toolkit::{
        inventory::{facts_command, parse_facts},
        setup::{self, SetupStep},
    },
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use uuid::Uuid;
type Result<T> = std::result::Result<T, PocError>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PackageAction {
    Enable,
    ResetEnabled,
    Disable,
    ForceStop,
    Remove,
    Uninstall,
    Restore,
    ClearData,
    ClearCache,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Action {
    AppOp {
        package: String,
        op: String,
        mode: String,
    },
    Package {
        package: String,
        action: PackageAction,
    },
    Shell {
        command: String,
    },
    Setting {
        namespace: String,
        key: String,
        value: Option<String>,
    },
    DefaultHome {
        component: String,
    },
    /// Turns a disabled launcher back on and makes it the default, in one step. `enable` lists
    /// other launchers to turn back on, which is how a switch is undone.
    RestoreHome {
        component: String,
        #[serde(default)]
        enable: Vec<String>,
    },
    /// Hands Home to a launcher: makes it the default, opens it, and turns off every launcher in
    /// `replacing`. One left on can take Home back, as Projectivy's override does.
    SwitchHome {
        component: String,
        replacing: Vec<String>,
    },
    Permission {
        package: String,
        permission: String,
        grant: bool,
    },
    FileDelete {
        path: String,
    },
    FileMove {
        from: String,
        to: String,
    },
    FileCreateFolder {
        path: String,
    },
    Setup {
        step: SetupStep,
    },
    Mappings {
        change: MappingChange,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PackageState {
    pub version: Option<u64>,
    pub installed: Option<bool>,
    pub enabled: Option<u8>,
    pub stopped: Option<bool>,
}
/// Android reports an app that came with the TV as SYSTEM, even after it has been updated.
pub fn is_system_app(raw: &str) -> bool {
    raw.lines()
        .find_map(|l| l.trim().strip_prefix("pkgFlags=["))
        .is_some_and(|flags| flags.split_whitespace().any(|f| f == "SYSTEM"))
}
async fn package_report(d: &mut dyn ShellDriver, t: &Target, package: &str) -> Result<String> {
    let output = d
        .shell(t, &format!("dumpsys package {}", package_arg(package)?))
        .await?;
    if !output.success() {
        return Err(PocError::new(
            "inspection",
            format!("Cannot inspect the current state: {}", output.combined()),
        ));
    }
    Ok(output.stdout)
}
pub fn parse_package_state(raw: &str, user: u32) -> PackageState {
    let version = raw
        .split("versionCode=")
        .nth(1)
        .and_then(|v| v.split_whitespace().next())
        .and_then(|v| v.parse().ok());
    let prefix = format!("User {user}:");
    let line = raw
        .lines()
        .find(|l| l.trim_start().starts_with(&prefix))
        .unwrap_or("");
    let field = |key: &str| line.split_whitespace().find_map(|v| v.strip_prefix(key));
    PackageState {
        version,
        installed: field("installed=").and_then(|v| v.parse().ok()),
        enabled: field("enabled=").and_then(|v| v.parse().ok()),
        stopped: field("stopped=").and_then(|v| v.parse().ok()),
    }
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Review {
    pub id: String,
    pub target: Target,
    pub title: String,
    pub effect: String,
    pub current_state: String,
    pub losses: Vec<String>,
    pub reversible: bool,
    pub commands: Vec<String>,
    pub expires_ms: i64,
    pub rating: Option<Level>,
}
#[derive(Serialize, Deserialize)]
struct Recovery {
    action: Action,
    expected_state: String,
}
#[derive(Clone)]
struct Prepared {
    review: Review,
    action: Action,
    before: String,
    recovery: Option<Action>,
}
pub struct Workspace {
    pub history: History,
    plans: Mutex<HashMap<String, Prepared>>,
    ratings: Option<Arc<Ratings>>,
}
impl Workspace {
    pub fn new(history: History) -> Self {
        Self {
            history,
            plans: Mutex::new(HashMap::new()),
            ratings: None,
        }
    }
    pub fn with_ratings(mut self, ratings: Arc<Ratings>) -> Self {
        self.ratings = Some(ratings);
        self
    }
    pub fn package_rating(&self, package: &str) -> Option<Level> {
        self.ratings
            .as_ref()?
            .current()?
            .get(package)
            .and_then(|r| r.level)
    }
    fn rating_of(&self, action: &Action) -> Option<Level> {
        match action {
            // A warning about losing an app would only put someone off getting it back.
            Action::Package {
                action: PackageAction::Enable | PackageAction::ResetEnabled | PackageAction::Restore,
                ..
            } => None,
            Action::Package { package, .. } => self.package_rating(package),
            _ => None,
        }
    }
    pub async fn prepare(
        &self,
        driver: &mut dyn ShellDriver,
        target: &Target,
        action: Action,
    ) -> Result<Review> {
        self.prepare_with(driver, target, action, false).await
    }
    /// Only a direct request from the app's detail view passes `allow_unsafe`; bulk and
    /// recovery go through `prepare`, so they can never skip the block.
    pub async fn prepare_with(
        &self,
        driver: &mut dyn ShellDriver,
        target: &Target,
        action: Action,
        allow_unsafe: bool,
    ) -> Result<Review> {
        validate_target(driver.current_target().as_ref(), target)?;
        let rating = self.rating_of(&action);
        if !allow_unsafe
            && rating == Some(Level::Unsafe)
            && matches!(
                action,
                Action::Package {
                    action: PackageAction::Disable
                        | PackageAction::Remove
                        | PackageAction::Uninstall
                        | PackageAction::ClearData,
                    ..
                }
            )
        {
            return Err(PocError::new(
                "unsafeRating",
                "The community list rates this app Unsafe: changing it can break vital parts of Android.",
            ));
        }
        let (title, effect, mut losses, command) = describe(&action, target.profile_id)?;
        let before = probe(driver, target, &action).await?;
        let mut recovery = None;
        if !file_paths(&action).is_empty() {
            let states: Vec<files::PathState> = serde_json::from_str(&before).map_err(internal)?;
            match &action {
                Action::FileDelete { path } => {
                    let state = files::state_of(&states, path)?;
                    files::check_delete(state)?;
                    losses.push(files::delete_loss(state));
                }
                Action::FileMove { from, to } => {
                    files::check_move(
                        files::state_of(&states, from)?,
                        files::state_of(&states, to)?,
                    )?;
                    recovery = Some(Action::FileMove {
                        from: to.clone(),
                        to: from.clone(),
                    });
                }
                Action::FileCreateFolder { path } => {
                    files::check_create(files::state_of(&states, path)?)?;
                }
                _ => {}
            }
        }
        if let Action::Package {
            package,
            action: kind,
        } = &action
        {
            let state: PackageState = serde_json::from_str(&before).map_err(internal)?;
            if state.installed.is_none() {
                return Err(PocError::new(
                    "unsupported",
                    "The TV did not report this package's state for the selected profile.",
                ));
            }
            if matches!(kind, PackageAction::Uninstall) {
                let raw = package_report(driver, target, package).await?;
                if is_system_app(&raw) {
                    return Err(PocError::new(
                        "protected",
                        "This app came with the TV, so Android won't uninstall it. Disable it, or use Remove from profile.",
                    ));
                }
            }
            if matches!(
                kind,
                PackageAction::Disable
                    | PackageAction::Remove
                    | PackageAction::Uninstall
                    | PackageAction::ClearData
            ) {
                protect_home(driver, target, package, kind).await?;
            }
            match kind {
                PackageAction::Enable | PackageAction::Disable | PackageAction::ResetEnabled => {
                    let inverse = match state.enabled {
                        Some(0) => Some(PackageAction::ResetEnabled),
                        Some(1) => Some(PackageAction::Enable),
                        Some(3) => Some(PackageAction::Disable),
                        _ => None,
                    };
                    recovery = inverse.map(|action| Action::Package {
                        package: package.clone(),
                        action,
                    });
                }
                // Removed application data is never represented as recoverable by reinstalling.
                _ => {}
            }
        }
        if let Action::Mappings {
            change: MappingChange::Apply { .. },
        } = &action
        {
            let prior: MappingSnapshot = serde_json::from_str(&before).map_err(internal)?;
            recovery = Some(Action::Mappings {
                change: if prior.mappings.is_empty() {
                    MappingChange::RemoveAll
                } else {
                    MappingChange::Apply {
                        mappings: prior.mappings,
                    }
                },
            });
        }
        if let Action::Setting { namespace, key, .. } = &action {
            recovery = Some(Action::Setting {
                namespace: namespace.clone(),
                key: key.clone(),
                value: if before == "null" {
                    None
                } else {
                    Some(before.clone())
                },
            });
        }
        if let Action::AppOp { package, op, .. } = &action {
            let prior: super::app_ops::Mode = serde_json::from_str(&before).map_err(internal)?;
            recovery = Some(Action::AppOp {
                package: package.clone(),
                op: op.clone(),
                mode: prior.package_mode,
            });
        }
        if let Action::Permission {
            package,
            permission,
            ..
        } = &action
        {
            recovery = Some(Action::Permission {
                package: package.clone(),
                permission: permission.clone(),
                grant: before == "granted",
            });
        }
        if matches!(
            action,
            Action::DefaultHome { .. } | Action::RestoreHome { .. }
        ) && component_arg(&before).is_ok()
        {
            recovery = Some(Action::DefaultHome {
                component: before.clone(),
            });
        }
        if let Action::SwitchHome { replacing, .. } = &action {
            let home = before.lines().next().unwrap_or_default();
            if component_arg(home).is_ok() {
                let package = home.split('/').next().unwrap_or_default();
                recovery = Some(Action::RestoreHome {
                    component: home.to_owned(),
                    enable: replacing.iter().filter(|p| *p != package).cloned().collect(),
                });
            }
        }
        let review = Review {
            id: Uuid::new_v4().to_string(),
            target: target.clone(),
            title,
            effect,
            current_state: state_label(&action, &before),
            losses,
            reversible: recovery.is_some(),
            commands: vec![command],
            expires_ms: now_ms() + 600_000,
            rating,
        };
        let mut plans = self.plans.lock().map_err(internal)?;
        plans.retain(|_, p| p.review.expires_ms > now_ms());
        if plans.len() >= 64 {
            return Err(PocError::new(
                "busy",
                "Too many open reviews. Finish or dismiss a review first.",
            ));
        }
        plans.insert(
            review.id.clone(),
            Prepared {
                review: review.clone(),
                action,
                before,
                recovery,
            },
        );
        Ok(review)
    }
    pub fn discard(&self, id: &str) -> Result<()> {
        self.plans.lock().map_err(internal)?.remove(id);
        Ok(())
    }
    pub async fn apply(&self, driver: &mut dyn ShellDriver, id: &str) -> Result<OperationRecord> {
        // Consume before dispatch. A timeout or duplicate IPC can never resend a mutation.
        let prepared = self
            .plans
            .lock()
            .map_err(internal)?
            .remove(id)
            .ok_or_else(|| {
                PocError::new(
                    "staleReview",
                    "This review was already used or dismissed. Review the action again.",
                )
            })?;
        let p = &prepared;
        let target = &p.review.target;
        validate_target(driver.current_target().as_ref(), target)?;
        if now_ms() > p.review.expires_ms {
            return Err(PocError::new(
                "staleReview",
                "This review expired. Inspect the current state again.",
            ));
        }
        if probe(driver, target, &p.action).await? != p.before {
            return Err(PocError::new(
                "staleReview",
                "The TV's state changed after review. Review the action again.",
            ));
        }
        if let Action::Package { package, action } = &p.action
            && matches!(
                action,
                PackageAction::Disable
                    | PackageAction::Remove
                    | PackageAction::Uninstall
                    | PackageAction::ClearData
            )
        {
            protect_home(driver, target, package, action).await?;
        }
        let now = now_ms();
        let mut row = OperationRecord {
            id: Uuid::new_v4().to_string(),
            target: target.clone(),
            title: p.review.title.clone(),
            device_label: driver.target_label(),
            kind: kind(&p.action).into(),
            status: OperationStatus::Running,
            created_ms: now,
            updated_ms: now,
            output: String::new(),
            error: None,
            recovery: None,
        };
        self.history.put(&row)?;
        match driver.shell(target, &p.review.commands[0]).await {
            Err(e) => {
                row.status = OperationStatus::Unknown;
                row.error = Some(format!("{}. Inspect the TV before retrying.", e.message));
            }
            Ok(output) => {
                row.output = if matches!(p.action, Action::Shell { .. }) {
                    format!("$ {}\n\n{}", p.review.commands[0], output.combined())
                } else {
                    output.combined()
                };
                if !output.success() {
                    row.status = OperationStatus::Failed;
                    row.error = Some(match output.exit_code {
                        Some(code) => format!("The TV reported exit code {code}."),
                        None => "The TV didn't report how the command ended.".into(),
                    });
                } else if matches!(
                    p.action,
                    Action::Shell { .. }
                        | Action::Package {
                            action: PackageAction::ClearData | PackageAction::ClearCache,
                            ..
                        }
                ) || matches!(p.action, Action::Setup { step } if !setup::has_readback(step))
                {
                    row.status = OperationStatus::Sent;
                } else {
                    match probe(driver, target, &p.action).await {
                        Err(e) => {
                            row.status = OperationStatus::Unknown;
                            row.error =
                                Some(format!("Command accepted; readback failed: {}", e.message));
                        }
                        Ok(after) => {
                            row.status = if verified(&p.action, &after) {
                                OperationStatus::Verified
                            } else {
                                OperationStatus::Unknown
                            };
                            if row.status == OperationStatus::Unknown {
                                row.error = Some(match &p.action {
                                    Action::SwitchHome { component, .. }
                                        if after.lines().next() == Some(component) =>
                                    {
                                        format!(
                                            "{} is the default now, but {} is still on.",
                                            component.split('/').next().unwrap_or_default(),
                                            after.lines().skip(1).collect::<Vec<_>>().join(", ")
                                        )
                                    }
                                    Action::DefaultHome { .. }
                                    | Action::RestoreHome { .. }
                                    | Action::SwitchHome { .. }
                                        if !after.is_empty() =>
                                    format!(
                                        "Android accepted this choice, but Home still opens {}.",
                                        after.split('/').next().unwrap_or_default()
                                    ),
                                    _ => "The command returned, but its expected effect was not verified. Inspect the TV before retrying.".into(),
                                });
                            } else {
                                row.recovery = p
                                    .recovery
                                    .as_ref()
                                    .map(|action| {
                                        serde_json::to_value(Recovery {
                                            action: action.clone(),
                                            expected_state: after.clone(),
                                        })
                                    })
                                    .transpose()
                                    .map_err(internal)?;
                            }
                        }
                    }
                }
            }
        }
        row.updated_ms = now_ms();
        self.history.put(&row)?;
        Ok(row)
    }
    pub async fn prepare_recovery(&self, driver: &mut dyn ShellDriver, id: &str) -> Result<Review> {
        let row = self.history.get(id)?;
        let current = driver
            .current_target()
            .ok_or_else(|| PocError::new("connection", "Connect to the original device first."))?;
        if current.device_id != row.target.device_id || current.profile_id != row.target.profile_id
        {
            return Err(PocError::new(
                "staleTarget",
                "Recovery belongs to a different device or profile.",
            ));
        }
        let recovery: Recovery = serde_json::from_value(row.recovery.ok_or_else(|| {
            PocError::new(
                "unsupported",
                "No supported inverse was recorded for this action.",
            )
        })?)
        .map_err(internal)?;
        if probe(driver, &current, &recovery.action).await? != recovery.expected_state {
            return Err(PocError::new(
                "staleRecovery",
                "The TV state changed after this operation. Its recorded recovery no longer applies; inspect the current state and prepare a new action.",
            ));
        }
        self.prepare(driver, &current, recovery.action).await
    }
}
fn state_label(action: &Action, before: &str) -> String {
    match action {
        Action::AppOp { .. } => serde_json::from_str::<super::app_ops::Mode>(before)
            .map(|s| {
                format!(
                    "Stored package mode: {}. UID override: {}.",
                    s.package_mode,
                    s.uid_mode.as_deref().unwrap_or("none reported")
                )
            })
            .unwrap_or_else(|_| "Policy unknown".into()),
        Action::Package { .. } => {
            if let Ok(s) = serde_json::from_str::<PackageState>(before) {
                let enabled = match s.enabled {
                    Some(0) => "default",
                    Some(1) => "enabled",
                    Some(2..=4) => "disabled",
                    _ => "unknown",
                };
                format!(
                    "Currently {} for this profile; enabled state: {enabled}; version {}.",
                    if s.installed == Some(true) {
                        "installed"
                    } else if s.installed == Some(false) {
                        "removed"
                    } else {
                        "unknown"
                    },
                    s.version
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "unknown".into())
                )
            } else {
                "Current package state unavailable.".into()
            }
        }
        Action::Shell { .. } => {
            "Arbitrary shell commands have no general readback or automatic undo.".into()
        }
        Action::Setup { .. } => "Checked on the TV just now.".into(),
        Action::Mappings { .. } => serde_json::from_str::<MappingSnapshot>(before)
            .map(|s| match (s.mappings.len(), s.paused) {
                (0, _) => "No button mappings on this TV yet.".to_owned(),
                (n, true) => format!("{n} saved, paused."),
                (n, false) => format!("{n} saved."),
            })
            .unwrap_or_else(|_| "Current state unavailable.".into()),
        Action::Permission { .. } => format!("Current permission: {before}"),
        Action::FileDelete { .. } | Action::FileMove { .. } | Action::FileCreateFolder { .. } => {
            serde_json::from_str::<Vec<files::PathState>>(before)
                .map(|states| files::state_summary(&states))
                .unwrap_or_else(|_| "Current state unavailable.".into())
        }
        Action::SwitchHome { .. } => format!(
            "Current value: {}",
            before.lines().next().unwrap_or_default()
        ),
        _ => format!("Current value: {before}"),
    }
}

fn internal(e: impl std::fmt::Display) -> PocError {
    PocError::new("workspace", e.to_string())
}
fn kind(a: &Action) -> &str {
    match a {
        Action::AppOp { .. } => "appOp",
        Action::Package { .. } => "package",
        Action::Shell { .. } => "shell",
        Action::Setting { .. } => "setting",
        Action::DefaultHome { .. } | Action::RestoreHome { .. } | Action::SwitchHome { .. } => {
            "launcher"
        }
        Action::Permission { .. } => "permission",
        Action::FileDelete { .. } | Action::FileMove { .. } | Action::FileCreateFolder { .. } => {
            "file"
        }
        Action::Setup { .. } => "setup",
        Action::Mappings { .. } => "mapping",
    }
}
fn file_paths(a: &Action) -> Vec<&str> {
    match a {
        Action::FileDelete { path } | Action::FileCreateFolder { path } => vec![path],
        Action::FileMove { from, to } => vec![from, to],
        _ => Vec::new(),
    }
}
fn home_query(user: u32) -> String {
    format!(
        "cmd package resolve-activity --brief --user {user} -a android.intent.action.MAIN -c android.intent.category.HOME"
    )
}
async fn protect_home(
    d: &mut dyn ShellDriver,
    t: &Target,
    package: &str,
    action: &PackageAction,
) -> Result<()> {
    if [
        "android",
        "com.android.systemui",
        "com.android.shell",
        "com.android.settings",
        "com.android.tv.settings",
        "com.android.permissioncontroller",
        "com.google.android.permissioncontroller",
    ]
    .contains(&package)
    {
        return Err(PocError::new(
            "protected",
            "This system component is protected from this operation.",
        ));
    }
    let home = d.shell(t, &home_query(t.profile_id)).await?;
    if !home.success() {
        return Err(PocError::new(
            "unsupported",
            "Cannot establish the active Home app. This action is unavailable until Home can be inspected.",
        ));
    }
    let component = home
        .stdout
        .lines()
        .find(|v| component_arg(v.trim()).is_ok())
        .ok_or_else(|| {
            PocError::new(
                "unsupported",
                "No unambiguous active Home app was reported.",
            )
        })?;
    if component.trim().split('/').next() == Some(package) {
        // A launcher with a higher priority outranks the one chosen, so turning it off is the
        // only way to hand over. Android keeps its data, and it can be turned back on.
        if matches!(action, PackageAction::Disable)
            && super::queries::has_replacement_home(d, t, package).await?
        {
            return Ok(());
        }
        return Err(PocError::new(
            "protected",
            "Choose and verify a replacement Home app before changing the active launcher.",
        ));
    }
    Ok(())
}
async fn probe(d: &mut dyn ShellDriver, t: &Target, a: &Action) -> Result<String> {
    if let Action::Permission {
        package,
        permission,
        ..
    } = a
    {
        package_arg(package)?;
        permission_arg(permission)?;
        let rows = d
            .metadata(t, std::slice::from_ref(package))
            .await?
            .ok_or_else(|| {
                PocError::new(
                    "unsupported",
                    "This TV cannot inspect runtime permissions through the available route.",
                )
            })?;
        let entry = rows
            .iter()
            .find(|m| &m.package == package)
            .and_then(|m| m.permissions.iter().find(|p| &p.name == permission))
            .ok_or_else(|| {
                PocError::new(
                    "permission",
                    "This permission is not declared by the selected app.",
                )
            })?;
        if entry.protection != Some(1) {
            return Err(PocError::new(
                "permission",
                "Only declared runtime permissions can be changed here. Signature and system permissions are unavailable.",
            ));
        }
        return Ok(if entry.granted { "granted" } else { "denied" }.into());
    }
    let paths = file_paths(a);
    if !paths.is_empty() {
        let output = d.shell(t, &files::state_command(&paths)).await?;
        return serde_json::to_string(&files::parse_states(&output.stdout)?).map_err(internal);
    }
    let command = match a {
        Action::AppOp { package, op, .. } => {
            super::app_ops::probe_command(t.profile_id, package, op)?
        }
        Action::Package { package, .. } => format!("dumpsys package {}", package_arg(package)?),
        Action::Setting { namespace, key, .. } => {
            validate_setting(namespace, key, None)?;
            format!(
                "settings --user {} get {} {}",
                t.profile_id,
                namespace,
                shell_quote(key)
            )
        }
        Action::DefaultHome { .. } | Action::RestoreHome { .. } => home_query(t.profile_id),
        Action::SwitchHome { .. } => format!(
            "{}; pm list packages -d --user {}",
            home_query(t.profile_id),
            t.profile_id
        ),
        Action::Setup { .. } => facts_command().to_owned(),
        Action::Mappings { .. } => mappings::state_command(),
        Action::Shell { .. } => return Ok(String::new()),
        Action::Permission { .. }
        | Action::FileDelete { .. }
        | Action::FileMove { .. }
        | Action::FileCreateFolder { .. } => unreachable!(),
    };
    let output = d.shell(t, &command).await?;
    if !output.success() {
        return Err(PocError::new(
            "inspection",
            format!("Cannot inspect the current state: {}", output.combined()),
        ));
    }
    match a {
        Action::AppOp { op, .. } => {
            serde_json::to_string(&super::app_ops::parse(&output.stdout, op)?).map_err(internal)
        }
        Action::Package { package, .. } => {
            let mut state = parse_package_state(&output.stdout, t.profile_id);
            // Once a package is really gone there is no section for it. Android 9 then prints its
            // general report, which has a "User 0:" line of its own.
            if state.installed.is_none() && !output.stdout.contains(&format!("Package [{package}]"))
            {
                state.installed = Some(false);
            }
            serde_json::to_string(&state).map_err(internal)
        }
        Action::Setup { .. } => Ok(parse_facts(&output.stdout).summary()),
        Action::Mappings { .. } => serde_json::to_string(&MappingSnapshot::from(
            mappings::parse_state(&output.stdout),
        ))
        .map_err(internal),
        Action::SwitchHome { replacing, .. } => {
            let home = output
                .stdout
                .lines()
                .find(|v| component_arg(v.trim()).is_ok())
                .unwrap_or("")
                .trim()
                .to_owned();
            let still_on = replacing.iter().filter(|old| {
                let wanted = format!("package:{old}");
                !output.stdout.lines().any(|l| l.trim() == wanted)
            });
            Ok(std::iter::once(home.as_str())
                .chain(still_on.map(String::as_str))
                .collect::<Vec<_>>()
                .join("\n"))
        }
        Action::DefaultHome { .. } | Action::RestoreHome { .. } => Ok(output
            .stdout
            .lines()
            .find(|v| component_arg(v.trim()).is_ok())
            .unwrap_or("")
            .trim()
            .into()),
        _ => Ok(output.stdout.trim().into()),
    }
}
fn verified(a: &Action, after: &str) -> bool {
    match a {
        Action::AppOp { mode, .. } => serde_json::from_str::<super::app_ops::Mode>(after)
            .is_ok_and(|s| &s.package_mode == mode),
        Action::Package { action, .. } => {
            let Ok(s) = serde_json::from_str::<PackageState>(after) else {
                return false;
            };
            match action {
                PackageAction::Enable => s.enabled == Some(1),
                PackageAction::ResetEnabled => s.enabled == Some(0),
                PackageAction::Disable => s.enabled == Some(3),
                PackageAction::Remove | PackageAction::Uninstall => s.installed == Some(false),
                PackageAction::Restore => s.installed == Some(true),
                PackageAction::ForceStop => s.stopped == Some(true),
                PackageAction::ClearData | PackageAction::ClearCache => false,
            }
        }
        Action::Setting { value, .. } => after == value.as_deref().unwrap_or("null"),
        Action::DefaultHome { component }
        | Action::RestoreHome { component, .. }
        | Action::SwitchHome { component, .. } => after == component,
        Action::Shell { .. } => false,
        Action::Permission { grant, .. } => after == if *grant { "granted" } else { "denied" },
        Action::FileDelete { path } | Action::FileCreateFolder { path } => {
            let exists = matches!(a, Action::FileCreateFolder { .. });
            serde_json::from_str::<Vec<files::PathState>>(after)
                .ok()
                .as_deref()
                .and_then(|states| files::state_of(states, path).ok())
                .is_some_and(|state| state.exists == exists)
        }
        Action::Setup { step } => setup::verified(*step, &parse_facts(after)),
        Action::Mappings { change } => serde_json::from_str::<MappingSnapshot>(after)
            .is_ok_and(|state| mappings::verified(change, &state)),
        Action::FileMove { from, to } => serde_json::from_str::<Vec<files::PathState>>(after)
            .ok()
            .as_deref()
            .is_some_and(|states| {
                files::state_of(states, from).is_ok_and(|s| !s.exists)
                    && files::state_of(states, to).is_ok_and(|s| s.exists)
            }),
    }
}
fn validate_setting(namespace: &str, key: &str, value: Option<&str>) -> Result<()> {
    let supported = matches!(
        (namespace, key),
        (
            "global",
            "window_animation_scale" | "transition_animation_scale" | "animator_duration_scale"
        ) | ("system", "screen_brightness" | "volume_music")
    );
    if !supported {
        return Err(PocError::new(
            "input",
            "Choose a setting from the supported catalog.",
        ));
    }
    if let Some(v) = value {
        let valid = if key.ends_with("_scale") {
            ["0", "0.5", "1", "1.5", "2", "5", "10"].contains(&v)
        } else {
            v.parse::<u16>().is_ok_and(|n| n <= 255)
        };
        if !valid {
            return Err(PocError::new("input", "Choose a supported setting value."));
        }
    }
    Ok(())
}
fn describe(a: &Action, user: u32) -> Result<(String, String, Vec<String>, String)> {
    let mut losses = Vec::new();
    let result = match a {
        Action::AppOp { package, op, mode } => {
            let command = super::app_ops::command(user, package, op, mode)?;
            losses.push("Special access can expose private activity, allow overlays or change background behavior. Android permissions, UID policy and firmware may override the stored package mode.".into());
            (
                format!("Change {op} for {package}"),
                format!(
                    "Set the stored policy to {mode} for profile {user}. Readback verifies the stored policy, not effective access."
                ),
                command,
            )
        }
        Action::Permission {
            package,
            permission,
            grant,
        } => {
            let package_quoted = package_arg(package)?;
            let permission_quoted = permission_arg(permission)?;
            losses.push(if *grant{"This gives the app access covered by the selected permission in this profile. Grant only access you intend the app to have."}else{"The app may stop working correctly until this permission is restored."}.into());
            (
                format!("{} {permission}", if *grant { "Grant" } else { "Revoke" }),
                format!(
                    "Change this runtime permission for {package}, profile {user}. Android policy may refuse the change."
                ),
                format!(
                    "pm {} --user {user} {package_quoted} {permission_quoted}",
                    if *grant { "grant" } else { "revoke" }
                ),
            )
        }
        Action::Package { package, action } => {
            let p = package_arg(package)?;
            let (title, command, effect) = match action {
                PackageAction::Enable => (
                    "Enable app",
                    format!("pm enable --user {user} {p}"),
                    "Allow the app to run in this profile.",
                ),
                PackageAction::ResetEnabled => (
                    "Restore default app state",
                    format!("pm default-state --user {user} {p}"),
                    "Let the package's default enabled state apply.",
                ),
                PackageAction::Disable => (
                    "Disable app",
                    format!("pm disable-user --user {user} {p}"),
                    "Hide and stop the app in this profile. Its data stays on the TV.",
                ),
                PackageAction::ForceStop => (
                    "Force stop app",
                    format!("am force-stop --user {user} {p}"),
                    "Stop the app and its current playback or background work.",
                ),
                PackageAction::Remove => {
                    losses.push("App data for this profile will be deleted. Installing the APK again does not restore it.".into());
                    (
                        "Remove app from profile",
                        format!("pm uninstall --user {user} {p}"),
                        "Remove this app for the selected profile. Other profiles are not requested to change.",
                    )
                }
                PackageAction::Uninstall => {
                    losses.push("The app and all its data are deleted from every profile. To get it back you have to install it again, and its data doesn't come back.".into());
                    if ["com.termux", "com.termux.boot"].contains(&package.as_str()) {
                        losses.push("Bridgecoil uses this app to keep your button mappings running. They stop working until you set it up again.".into());
                    }
                    (
                        "Uninstall app",
                        format!("pm uninstall {p}"),
                        "Delete this app from the TV completely.",
                    )
                }
                PackageAction::Restore => (
                    "Restore existing package",
                    format!("cmd package install-existing --user {user} {p}"),
                    "Re-add a package that remains on the TV. Deleted app data is not restored.",
                ),
                PackageAction::ClearData => {
                    losses.push("Sign-ins, settings, downloads and other app data in this profile will be deleted.".into());
                    (
                        "Clear app data",
                        format!("pm clear --user {user} {p}"),
                        "Reset the app for this profile. This cannot be undone.",
                    )
                }
                PackageAction::ClearCache => (
                    "Clear app cache",
                    format!("pm clear --cache-only --user {user} {p}"),
                    "Ask Android to clear the app's cache. Older firmware may not support this option.",
                ),
            };
            (format!("{title} · {package}"), effect.into(), command)
        }
        Action::Shell { command } => {
            if command.trim().is_empty() || command.len() > 8192 || command.contains('\0') {
                return Err(PocError::new(
                    "input",
                    "Enter a nonempty command up to 8 KiB.",
                ));
            }
            losses.push("Shell commands can change the whole TV, including other profiles. There is no automatic undo. Do not include passwords or pairing codes; command text is retained in local history.".into());
            (
                "Run shell command".into(),
                "Run exactly the command shown using the connected ADB shell identity.".into(),
                command.clone(),
            )
        }
        Action::Setting {
            namespace,
            key,
            value,
        } => {
            validate_setting(namespace, key, value.as_deref())?;
            let command = if let Some(v) = value {
                format!(
                    "settings --user {user} put {namespace} {} {}",
                    shell_quote(key),
                    shell_quote(v)
                )
            } else {
                format!(
                    "settings --user {user} delete {namespace} {}",
                    shell_quote(key)
                )
            };
            (
                format!("Change {key}"),
                if namespace == "global" {
                    "This is a TV-wide setting and affects every profile."
                } else {
                    "Change the selected profile's setting; firmware may ignore this value."
                }
                .into(),
                command,
            )
        }
        Action::FileDelete { path } => {
            let path = files::action_path(path)?;
            (
                format!("Delete {}", base_name(path)),
                format!("Delete {path} from the TV."),
                format!("rm -rf {}", shell_quote(path)),
            )
        }
        Action::FileMove { from, to } => {
            let (from, to) = (files::action_path(from)?, files::action_path(to)?);
            if from == to {
                return Err(PocError::new("input", "Choose a different name or folder."));
            }
            let renamed = parent_path(from) == parent_path(to);
            (
                format!(
                    "{} {}",
                    if renamed { "Rename" } else { "Move" },
                    base_name(from)
                ),
                format!("Change {from} to {to}."),
                format!("mv -n {} {}", shell_quote(from), shell_quote(to)),
            )
        }
        Action::FileCreateFolder { path } => {
            let path = files::action_path(path)?;
            (
                format!("Create folder {}", base_name(path)),
                format!("Create {path} on the TV."),
                format!("mkdir {}", shell_quote(path)),
            )
        }
        Action::Setup { step } => {
            let plan = setup::plan(*step);
            losses.extend(plan.losses);
            (plan.title, plan.effect, plan.command)
        }
        Action::SwitchHome {
            component,
            replacing,
        } => {
            let c = component_arg(component)?;
            let package = component.split('/').next().unwrap_or_default();
            let mut command = format!(
                "cmd package set-home-activity --user {user} {c} && am start -n {c}"
            );
            let mut effect = format!("Make {package} the default for this profile and open it.");
            for old in replacing {
                if old == package || old == "android" || old.ends_with(".settings") {
                    return Err(PocError::new(
                        "protected",
                        "That app can't be turned off to change launchers.",
                    ));
                }
                let p = package_arg(old)?;
                command.push_str(&format!(" && pm disable-user --user {user} {p}"));
                effect.push_str(&format!(" Turn off {old}. Its data stays on the TV."));
                losses.push(format!("Some TV features can depend on {old}."));
            }
            (
                format!("Switch Home app · {package}"),
                effect,
                command,
            )
        }
        Action::RestoreHome { component, enable } => {
            let c = component_arg(component)?;
            let package = component.split('/').next().unwrap_or_default();
            let mut command = String::new();
            for name in std::iter::once(package).chain(enable.iter().map(String::as_str)) {
                command.push_str(&format!("pm enable --user {user} {} && ", package_arg(name)?));
            }
            command.push_str(&format!("cmd package set-home-activity --user {user} {c}"));
            (
                format!("Restore launcher · {package}"),
                format!("Turn {package} back on and make it the default for this profile. Test the physical Home button afterwards."),
                command,
            )
        }
        Action::Mappings { change } => {
            let plan = mappings::plan(change)?;
            losses.extend(plan.losses);
            (plan.title, plan.effect, plan.command)
        }
        Action::DefaultHome { component } => {
            let c = component_arg(component)?;
            (format!("Set default Home app · {}", component.split('/').next().unwrap_or_default()),"Request this Home activity for the profile. OEM firmware may reject it, show a TV chooser or keep its own launcher first. Test the physical Home button afterwards.".into(),format!("cmd package set-home-activity --user {user} {c}"))
        }
    };
    Ok((result.0, result.1, losses, result.2))
}
fn base_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}
fn parent_path(path: &str) -> &str {
    path.rsplit_once('/').map_or(path, |(parent, _)| parent)
}
fn permission_arg(permission: &str) -> Result<String> {
    if permission.is_empty()
        || permission.len() > 256
        || !permission
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._".contains(c))
    {
        return Err(PocError::new(
            "input",
            "Choose a declared runtime permission.",
        ));
    }
    Ok(shell_quote(permission))
}
