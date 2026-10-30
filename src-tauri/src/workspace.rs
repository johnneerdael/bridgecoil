use crate::{Backend, Result, Selections, err};
use adb_core::{
    backend::{DeviceInfo, PocError},
    workspace::{
        bulk::{Bulk, BulkReview},
        engine::{Action, PackageAction, Review, Workspace},
        history::now_ms,
        queries::{self, Query},
        types::*,
    },
};
use serde::Deserialize;
use tauri::State;

#[tauri::command]
pub async fn prepare_bulk(
    target: Target,
    packages: Vec<String>,
    action: PackageAction,
    backend: State<'_, Backend>,
    workspace: State<'_, Workspace>,
    bulk: State<'_, Bulk>,
) -> Result<BulkReview> {
    bulk.prepare(
        &workspace,
        &mut *backend.0.lock().await,
        &target,
        packages,
        action,
    )
    .await
}
#[tauri::command]
pub async fn apply_bulk(
    review_id: String,
    stop_after_failure: bool,
    backend: State<'_, Backend>,
    workspace: State<'_, Workspace>,
    bulk: State<'_, Bulk>,
) -> Result<OperationRecord> {
    bulk.apply(
        &workspace,
        &mut *backend.0.lock().await,
        &review_id,
        stop_after_failure,
    )
    .await
}
#[tauri::command]
pub fn cancel_bulk(review_id: String, bulk: State<'_, Bulk>) -> Result<()> {
    bulk.cancel(&review_id)
}
#[tauri::command]
pub fn discard_bulk(
    review_id: String,
    workspace: State<'_, Workspace>,
    bulk: State<'_, Bulk>,
) -> Result<()> {
    bulk.discard(&workspace, &review_id)
}

#[tauri::command]
pub async fn workspace_query(
    target: Target,
    query: Query,
    backend: State<'_, Backend>,
) -> Result<serde_json::Value> {
    queries::query(&mut *backend.0.lock().await, &target, query).await
}
#[tauri::command]
pub async fn prepare_action(
    target: Target,
    action: Action,
    backend: State<'_, Backend>,
    workspace: State<'_, Workspace>,
) -> Result<Review> {
    workspace
        .prepare(&mut *backend.0.lock().await, &target, action)
        .await
}
#[tauri::command]
pub async fn apply_action(
    review_id: String,
    backend: State<'_, Backend>,
    workspace: State<'_, Workspace>,
) -> Result<OperationRecord> {
    workspace
        .apply(&mut *backend.0.lock().await, &review_id)
        .await
}
#[tauri::command]
pub fn discard_review(review_id: String, workspace: State<'_, Workspace>) -> Result<()> {
    workspace.discard(&review_id)
}
#[tauri::command]
pub fn list_history(
    page: usize,
    page_size: usize,
    workspace: State<'_, Workspace>,
) -> Result<Page<OperationRecord>> {
    workspace.history.list(page, page_size)
}
#[tauri::command]
pub fn clear_history(workspace: State<'_, Workspace>) -> Result<()> {
    workspace.history.clear()
}
#[tauri::command]
pub async fn prepare_recovery(
    operation_id: String,
    backend: State<'_, Backend>,
    workspace: State<'_, Workspace>,
) -> Result<Review> {
    workspace
        .prepare_recovery(&mut *backend.0.lock().await, &operation_id)
        .await
}
#[tauri::command]
pub async fn select_profile(
    target: Target,
    profile_id: u32,
    backend: State<'_, Backend>,
    selections: State<'_, Selections>,
) -> Result<DeviceInfo> {
    let mut client = backend.0.lock().await;
    let value = queries::query(&mut *client, &target, Query::Profiles).await?;
    let profiles: Vec<queries::Profile> = serde_json::from_value(value).map_err(err)?;
    if !profiles.iter().any(|p| p.id == profile_id) {
        return Err(PocError::new(
            "profile",
            "This profile is no longer available.",
        ));
    }
    let info = client.select_profile(&target, profile_id)?;
    selections.0.lock().map_err(err)?.clear();
    Ok(info)
}
#[derive(Debug, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum RemoteAction {
    Key { key: String, long_press: bool },
    Text { text: String },
    Launch { component: String },
}
#[tauri::command]
pub async fn remote_action(
    target: Target,
    action: RemoteAction,
    backend: State<'_, Backend>,
    workspace: State<'_, Workspace>,
) -> Result<OperationRecord> {
    use adb_core::{shell_quote, workspace::commands::component_arg};
    let mut client = backend.0.lock().await;
    validate_target(client.current_target().as_ref(), &target)?;
    let (title, command) = match action {
        RemoteAction::Key { key, long_press } => {
            let code = match key.as_str() {
                "up" => 19,
                "down" => 20,
                "left" => 21,
                "right" => 22,
                "ok" => 23,
                "home" => 3,
                "back" => 4,
                "menu" => 82,
                "volumeUp" => 24,
                "volumeDown" => 25,
                "mute" => 164,
                "playPause" => 85,
                "previous" => 88,
                "next" => 87,
                "power" => 26,
                _ => return Err(PocError::new("input", "Choose a supported remote key.")),
            };
            (
                format!("Remote · {key}"),
                format!(
                    "input keyevent {}{code}",
                    if long_press { "--longpress " } else { "" }
                ),
            )
        }
        RemoteAction::Text { text } => {
            if text.is_empty()
                || text.len() > 1024
                || !text
                    .chars()
                    .all(|c| c.is_ascii_graphic() && c != '%' || c == ' ')
            {
                return Err(PocError::new(
                    "unsupported",
                    "This TV text route accepts up to 1024 printable ASCII characters, excluding %. Use the TV keyboard for other characters.",
                ));
            }
            (
                "Send text".into(),
                format!("input text {}", shell_quote(&text.replace(' ', "%s"))),
            )
        }
        RemoteAction::Launch { component } => {
            component_arg(&component)?;
            let package = component.split('/').next().unwrap_or("");
            let detail = queries::query(
                &mut *client,
                &target,
                Query::Package {
                    package: package.into(),
                },
            )
            .await?;
            if !detail["activities"]
                .as_array()
                .is_some_and(|a| a.iter().any(|v| v.as_str() == Some(component.as_str())))
            {
                let homes = queries::query(&mut *client, &target, Query::Launchers).await?;
                if !homes["homes"]
                    .as_array()
                    .is_some_and(|a| a.iter().any(|v| v.as_str() == Some(component.as_str())))
                {
                    return Err(PocError::new(
                        "activity",
                        "The selected launch activity is no longer available.",
                    ));
                }
            }
            (
                format!("Launch {package}"),
                format!(
                    "am start --user {} -n {}",
                    target.profile_id,
                    component_arg(&component)?
                ),
            )
        }
    };
    let now = now_ms();
    let mut row = OperationRecord {
        id: uuid::Uuid::new_v4().to_string(),
        target: target.clone(),
        title,
        kind: "remote".into(),
        status: OperationStatus::Running,
        created_ms: now,
        updated_ms: now,
        output: String::new(),
        error: None,
        recovery: None,
    };
    workspace.history.put(&row)?;
    match client.execute_target(&target, &command).await {
        Ok(o) => {
            row.status = if o.success() {
                OperationStatus::Sent
            } else {
                OperationStatus::Failed
            };
            row.output = o.combined();
        }
        Err(e) => {
            row.status = OperationStatus::Unknown;
            row.error = Some(e.message);
        }
    }
    row.updated_ms = now_ms();
    workspace.history.put(&row)?;
    Ok(row)
}
