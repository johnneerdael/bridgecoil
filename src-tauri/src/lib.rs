mod local_network;
mod workspace;
use adb_core::{
    apk::{self, ApkInfo},
    backend::{Client, DeviceInfo, InstallProgress, PocError},
    workspace::files,
};
use serde::Serialize;
use std::{
    collections::HashMap,
    io::{Read, Write},
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::{Emitter, Manager, State, ipc::Channel};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_fs::FsExt;
use tempfile::NamedTempFile;
use workspace::*;

type Result<T> = std::result::Result<T, PocError>;
struct Selected {
    file: NamedTempFile,
    info: ApkInfo,
    session_id: String,
}
#[derive(Default)]
struct Selections(Mutex<HashMap<String, Selected>>);
struct Backend(tokio::sync::Mutex<Client>);
#[derive(Default)]
struct ConnectionAttempt(Mutex<Option<(String, tokio::sync::watch::Sender<bool>)>>);
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConnectionEvent {
    attempt_id: String,
    #[serde(flatten)]
    progress: adb_core::connection::ConnectionProgress,
}
fn begin_connection_attempt(
    state: &ConnectionAttempt,
) -> Result<(String, tokio::sync::watch::Receiver<bool>)> {
    let (sender, receiver) = tokio::sync::watch::channel(false);
    let id = uuid::Uuid::new_v4().to_string();
    *state.0.lock().map_err(err)? = Some((id.clone(), sender));
    Ok((id, receiver))
}
fn finish_connection_attempt(state: &ConnectionAttempt, id: &str) {
    if let Ok(mut current) = state.0.lock() {
        if current.as_ref().is_some_and(|(active, _)| active == id) {
            *current = None;
        }
    }
}
async fn await_connection_cancel(receiver: &mut tokio::sync::watch::Receiver<bool>) {
    loop {
        let cancelled = *receiver.borrow();
        if cancelled || receiver.changed().await.is_err() {
            break;
        }
    }
}
#[tauri::command]
fn cancel_connection(attempt_id: String, state: State<'_, ConnectionAttempt>) -> Result<()> {
    let current = state.0.lock().map_err(err)?;
    if let Some((active, sender)) = &*current {
        if active == &attempt_id {
            let _ = sender.send(true);
        }
    }
    Ok(())
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SelectionInfo {
    id: String,
    session_id: String,
    name: String,
    #[serde(flatten)]
    info: ApkInfo,
}
fn err(e: impl std::fmt::Display) -> PocError {
    PocError::new("file", e.to_string())
}
#[tauri::command]
async fn set_appearance(app: tauri::AppHandle, dark: bool) -> Result<()> {
    #[cfg(target_os = "android")]
    local_network::set_appearance(&app, dark).await?;
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    app.set_theme(Some(if dark {
        tauri::Theme::Dark
    } else {
        tauri::Theme::Light
    }));
    #[cfg(target_os = "ios")]
    let _ = (app, dark);
    Ok(())
}
#[tauri::command]
async fn connection_status(state: State<'_, Backend>) -> Result<Option<DeviceInfo>> {
    Ok(state.0.lock().await.status())
}
#[tauri::command]
async fn discover_adb(app: tauri::AppHandle) -> Result<adb_core::discovery::DiscoveryResult> {
    local_network::discover(&app).await
}
#[tauri::command]
async fn pair_device(
    app: tauri::AppHandle,
    endpoint: String,
    code: String,
    state: State<'_, Backend>,
    attempts: State<'_, ConnectionAttempt>,
    on_progress: Channel<ConnectionEvent>,
) -> Result<String> {
    local_network::ensure(&app).await?;
    let mut client = state.0.lock().await;
    let (id, mut cancel) = begin_connection_attempt(&attempts)?;
    {
        let _ = on_progress.send(ConnectionEvent {
            attempt_id: id.clone(),
            progress: adb_core::connection::ConnectionProgress {
                stage: adb_core::connection::ConnectionStage::Pairing,
            },
        });
    }
    let result = tokio::select! {result=client.pair(endpoint,code)=>result,_=await_connection_cancel(&mut cancel)=>Err(PocError::new("cancelled","Pairing cancelled."))};
    finish_connection_attempt(&attempts, &id);
    result
}
#[tauri::command]
async fn connect_device(
    app: tauri::AppHandle,
    endpoint: String,
    allow_legacy: bool,
    expected_device_id: Option<String>,
    state: State<'_, Backend>,
    selections: State<'_, Selections>,
    attempts: State<'_, ConnectionAttempt>,
    on_progress: Channel<ConnectionEvent>,
) -> Result<DeviceInfo> {
    local_network::ensure(&app).await?;
    selections.0.lock().map_err(err)?.clear();
    let mut client = state.0.lock().await;
    let (id, mut cancel) = begin_connection_attempt(&attempts)?;
    let progress_id = id.clone();
    let result = tokio::select! {
        result=client.connect_with_progress(endpoint,allow_legacy,move |progress|{{let _=on_progress.send(ConnectionEvent{attempt_id:progress_id.clone(),progress});}})=>result,
        _=await_connection_cancel(&mut cancel)=>Err(PocError::new("cancelled","Connection cancelled."))
    };
    finish_connection_attempt(&attempts, &id);
    if let Ok(device) = &result {
        if expected_device_id.as_ref().is_some_and(|expected| expected != &device.device_id) {
            client.disconnect();
            return Err(PocError::new("differentDevice", "That address belongs to a different device. Check the saved TV’s connection address."));
        }
    }
    result
}
#[tauri::command]
async fn disconnect_device(
    state: State<'_, Backend>,
    selections: State<'_, Selections>,
) -> Result<()> {
    state.0.lock().await.disconnect();
    selections.0.lock().map_err(err)?.clear();
    Ok(())
}
#[tauri::command]
async fn choose_apk(
    app: tauri::AppHandle,
    session_id: String,
    state: State<'_, Backend>,
    selections: State<'_, Selections>,
) -> Result<Option<SelectionInfo>> {
    if state
        .0
        .lock()
        .await
        .status()
        .is_none_or(|v| v.id != session_id)
    {
        return Err(PocError::new(
            "staleSession",
            "Connect to a device before choosing an APK.",
        ));
    }
    let picker_app = app.clone();
    let selected = tauri::async_runtime::spawn_blocking(
        move || -> Result<Option<(NamedTempFile, ApkInfo)>> {
            let Some(path) = picker_app
                .dialog()
                .file()
                .add_filter("Android package", &["apk"])
                .blocking_pick_file()
            else {
                return Ok(None);
            };
            let copied = (|| -> Result<NamedTempFile> {
                let mut opts = tauri_plugin_fs::OpenOptions::new();
                opts.read(true);
                let source = picker_app.fs().open(path.clone(), opts).map_err(err)?;
                let mut file =
                    NamedTempFile::new_in(picker_app.path().app_cache_dir().map_err(err)?)
                        .map_err(err)?;
                let mut bounded = source.take(apk::MAX_APK_BYTES + 1);
                let n = std::io::copy(&mut bounded, &mut file).map_err(err)?;
                if n == 0 || n > apk::MAX_APK_BYTES {
                    return Err(PocError::new(
                        "apk",
                        "Choose a nonempty APK smaller than 1 GiB.",
                    ));
                }
                file.flush().map_err(err)?;
                Ok(file)
            })();
            #[cfg(target_os = "ios")]
            let _ = picker_app
                .fs()
                .stop_accessing_security_scoped_resource(path);
            let file = copied?;
            let info = apk::inspect(file.path()).map_err(err)?;
            Ok(Some((file, info)))
        },
    )
    .await
    .map_err(err)??;
    let Some((file, info)) = selected else {
        return Ok(None);
    };
    if state
        .0
        .lock()
        .await
        .status()
        .is_none_or(|v| v.id != session_id)
    {
        return Err(PocError::new(
            "staleSession",
            "Connection changed while choosing the file. Choose it again after reconnecting.",
        ));
    }
    let id = uuid::Uuid::new_v4().to_string();
    let result = SelectionInfo {
        id: id.clone(),
        session_id: session_id.clone(),
        name: format!("{}-{}.apk", info.package, info.version_code),
        info: info.clone(),
    };
    let mut store = selections.0.lock().map_err(err)?;
    store.clear();
    store.insert(
        id,
        Selected {
            file,
            info,
            session_id,
        },
    );
    Ok(Some(result))
}
#[tauri::command]
async fn install_apk(
    app: tauri::AppHandle,
    session_id: String,
    selection_id: String,
    state: State<'_, Backend>,
    selections: State<'_, Selections>,
) -> Result<()> {
    let selected = {
        let mut store = selections.0.lock().map_err(err)?;
        if store
            .get(&selection_id)
            .is_none_or(|v| v.session_id != session_id)
        {
            return Err(PocError::new(
                "staleSelection",
                "Choose the APK again for this connection.",
            ));
        }
        store.remove(&selection_id).ok_or_else(|| {
            PocError::new("staleSelection", "This APK selection was already used.")
        })?
    };
    let progress_id = selection_id.clone();
    state
        .0
        .lock()
        .await
        .install(
            &session_id,
            selected.file.path(),
            &selected.info,
            move |p: InstallProgress| {
                let _ = app.emit("install-progress", serde_payload(&progress_id, p));
            },
        )
        .await
}
struct PendingUpload {
    id: String,
    session_id: String,
    file: std::fs::File,
    size: u64,
}
#[derive(Default)]
struct Uploads {
    pending: Mutex<Option<PendingUpload>>,
    cancel: AtomicBool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UploadChoice {
    id: String,
    name: String,
    size: u64,
}
#[tauri::command]
async fn choose_upload(
    app: tauri::AppHandle,
    session_id: String,
    state: State<'_, Backend>,
    uploads: State<'_, Uploads>,
) -> Result<Option<UploadChoice>> {
    if state
        .0
        .lock()
        .await
        .status()
        .is_none_or(|v| v.id != session_id)
    {
        return Err(PocError::new(
            "staleSession",
            "Connect to a device before choosing a file.",
        ));
    }
    let picker_app = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(
        move || -> Result<Option<(std::fs::File, String, u64)>> {
            let Some(path) = picker_app.dialog().file().blocking_pick_file() else {
                return Ok(None);
            };
            let name = files::picked_name(&path.to_string());
            let mut opts = tauri_plugin_fs::OpenOptions::new();
            opts.read(true);
            let opened = picker_app.fs().open(path.clone(), opts).map_err(err);
            #[cfg(target_os = "ios")]
            let _ = picker_app
                .fs()
                .stop_accessing_security_scoped_resource(path);
            let file = opened?;
            let size = file.metadata().map_err(err)?.len();
            Ok(Some((file, name, size)))
        },
    )
    .await
    .map_err(err)??;
    let Some((file, name, size)) = picked else {
        return Ok(None);
    };
    let id = uuid::Uuid::new_v4().to_string();
    *uploads.pending.lock().map_err(err)? = Some(PendingUpload {
        id: id.clone(),
        session_id,
        file,
        size,
    });
    Ok(Some(UploadChoice { id, name, size }))
}
#[tauri::command]
async fn start_upload(
    session_id: String,
    upload_id: String,
    folder: String,
    name: String,
    on_progress: Channel<u64>,
    state: State<'_, Backend>,
    uploads: State<'_, Uploads>,
) -> Result<()> {
    let remote = files::child_path(&folder, &name)?;
    let pending = {
        let mut slot = uploads.pending.lock().map_err(err)?;
        if slot
            .as_ref()
            .is_none_or(|p| p.id != upload_id || p.session_id != session_id)
        {
            return Err(PocError::new(
                "staleSelection",
                "Choose the file again for this connection.",
            ));
        }
        slot.take()
            .ok_or_else(|| PocError::new("staleSelection", "This file was already used."))?
    };
    uploads.cancel.store(false, Ordering::SeqCst);
    state
        .0
        .lock()
        .await
        .upload(
            &session_id,
            tokio::fs::File::from_std(pending.file),
            pending.size,
            &remote,
            &uploads.cancel,
            move |sent| {
                let _ = on_progress.send(sent);
            },
        )
        .await
}
#[tauri::command]
fn cancel_upload(uploads: State<'_, Uploads>) {
    uploads.cancel.store(true, Ordering::SeqCst);
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    selection_id: String,
    #[serde(flatten)]
    progress: InstallProgress,
}
fn serde_payload(id: &str, progress: InstallProgress) -> ProgressEvent {
    ProgressEvent {
        selection_id: id.into(),
        progress,
    }
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(local_network::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data = app.path().app_data_dir()?;
            let cache = app.path().app_cache_dir()?;
            std::fs::create_dir_all(&data)?;
            std::fs::create_dir_all(&cache)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&data, std::fs::Permissions::from_mode(0o700))?;
            }
            app.manage(Backend(tokio::sync::Mutex::new(Client::new(
                data.join("identity"),
            ))));
            app.manage(Selections::default());
            app.manage(Uploads::default());
            let ratings = std::sync::Arc::new(adb_core::ratings::store::Ratings::open(
                &data.join("ratings"),
            ));
            app.manage(ratings.clone());
            app.manage(ConnectionAttempt::default());
            app.manage(adb_core::workspace::bulk::Bulk::default());
            app.manage(
                adb_core::workspace::engine::Workspace::new(
                    adb_core::workspace::history::History::open(&data.join("history.sqlite3"))?,
                )
                .with_ratings(ratings),
            );
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            prepare_bulk,
            apply_bulk,
            cancel_bulk,
            discard_bulk,
            workspace_query,
            prepare_action,
            apply_action,
            discard_review,
            list_history,
            clear_history,
            prepare_recovery,
            select_profile,
            remote_action,
            set_appearance,
            cancel_connection,
            connection_status,
            discover_adb,
            pair_device,
            connect_device,
            disconnect_device,
            choose_apk,
            install_apk,
            choose_upload,
            start_upload,
            cancel_upload,
            ratings_status,
            ratings_update,
            ratings_update_if_due,
            ratings_remove
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Bridgecoil");
}
