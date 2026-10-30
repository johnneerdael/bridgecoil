use adb_core::{
    apk::{self, ApkInfo},
    backend::{Client, DeviceInfo, InstallProgress, PocError},
};
use serde::Serialize;
use std::{
    collections::HashMap,
    io::{Read, Write},
    sync::Mutex,
};
use tauri::{Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_fs::FsExt;
use tempfile::NamedTempFile;

type Result<T> = std::result::Result<T, PocError>;
struct Selected {
    file: NamedTempFile,
    info: ApkInfo,
    session_id: String,
}
#[derive(Default)]
struct Selections(Mutex<HashMap<String, Selected>>);
struct Backend(tokio::sync::Mutex<Client>);
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
async fn connection_status(state: State<'_, Backend>) -> Result<Option<DeviceInfo>> {
    Ok(state.0.lock().await.status())
}
#[tauri::command]
async fn pair_device(endpoint: String, code: String, state: State<'_, Backend>) -> Result<String> {
    state.0.lock().await.pair(endpoint, code).await
}
#[tauri::command]
async fn connect_device(
    endpoint: String,
    allow_legacy: bool,
    state: State<'_, Backend>,
    selections: State<'_, Selections>,
) -> Result<DeviceInfo> {
    selections.0.lock().map_err(err)?.clear();
    state.0.lock().await.connect(endpoint, allow_legacy).await
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
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            connection_status,
            pair_device,
            connect_device,
            disconnect_device,
            choose_apk,
            install_apk
        ])
        .run(tauri::generate_context!())
        .expect("failed to run ADB Studio");
}
