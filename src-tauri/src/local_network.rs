//! On-demand LAN permission and bounded platform-native mobile discovery.
use tauri::Manager;
use tauri::{
    AppHandle, Runtime,
    plugin::{Builder, TauriPlugin},
};
#[cfg(any(target_os = "android", target_os = "ios"))]
struct LanHandle<R: Runtime>(tauri::plugin::PluginHandle<R>);
#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_bridgecoil_network);
#[derive(Default)]
struct DiscoveryScan(tokio::sync::Mutex<()>);

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("local-network")
        .setup(|app, api| {
            app.manage(DiscoveryScan::default());
            #[cfg(target_os = "android")]
            {
                let handle =
                    api.register_android_plugin("nl.neerdael.bridgecoil", "LanAccessPlugin")?;
                app.manage(LanHandle(handle));
            }
            #[cfg(target_os = "ios")]
            {
                app.manage(LanHandle(
                    api.register_ios_plugin(init_plugin_bridgecoil_network)?,
                ));
            }
            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            let _ = (app, api);
            Ok(())
        })
        .build()
}
/// Android closes a background app's sockets, so hold on to the network while a TV is connected.
pub async fn keep_connection(app: &AppHandle, connected: bool) -> crate::Result<()> {
    #[cfg(target_os = "android")]
    {
        let handle = app.state::<LanHandle<tauri::Wry>>().0.clone();
        tauri::async_runtime::spawn_blocking(move || {
            handle.run_mobile_plugin::<serde_json::Value>(
                "keepConnection",
                serde_json::json!({ "connected": connected }),
            )
        })
        .await
        .map_err(|e| adb_core::backend::PocError::new("connection", e.to_string()))?
        .map_err(|e| adb_core::backend::PocError::new("connection", e.to_string()))?;
    }
    #[cfg(not(target_os = "android"))]
    let _ = (app, connected);
    Ok(())
}
pub async fn discover(app: &AppHandle) -> crate::Result<adb_core::discovery::DiscoveryResult> {
    ensure(app).await?;
    let state = app.state::<DiscoveryScan>();
    let _scan = state.0.try_lock().map_err(|_| {
        adb_core::backend::PocError::new("busy", "A discovery scan is already running.")
    })?;
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        let handle = app.state::<LanHandle<tauri::Wry>>().0.clone();
        let raw = tauri::async_runtime::spawn_blocking(move || {
            handle.run_mobile_plugin::<adb_core::discovery::RawDiscovery>(
                "discover",
                serde_json::json!({}),
            )
        })
        .await
        .map_err(|e| adb_core::backend::PocError::new("discovery", e.to_string()))?
        .map_err(|e| {
            let code = match &e {
                tauri::plugin::mobile::PluginInvokeError::InvokeRejected(response) => {
                    response.code.as_deref()
                }
                _ => None,
            };
            discovery_error(code, e.to_string())
        })?;
        Ok(adb_core::discovery::normalize(raw))
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    tauri::async_runtime::spawn_blocking(|| {
        adb_core::discovery::discover(std::time::Duration::from_secs(4))
    })
    .await
    .map_err(|e| adb_core::backend::PocError::new("discovery", e.to_string()))?
}
#[cfg(any(target_os = "android", target_os = "ios", test))]
fn discovery_error(code: Option<&str>, message: String) -> adb_core::backend::PocError {
    adb_core::backend::PocError::new(
        if code == Some("permission") {
            "permission"
        } else {
            "discovery"
        },
        message,
    )
}
#[cfg(any(target_os = "android", test))]
fn require_grant(granted: bool) -> crate::Result<()> {
    if granted {
        Ok(())
    } else {
        Err(adb_core::backend::PocError::new(
            "permission",
            "Allow local network access in Android Settings → Apps → Bridgecoil → Permissions, then retry.",
        ))
    }
}
pub async fn ensure(app: &AppHandle) -> crate::Result<()> {
    #[cfg(target_os = "android")]
    {
        #[derive(serde::Deserialize)]
        struct Access {
            granted: bool,
        }
        let handle = app.state::<LanHandle<tauri::Wry>>().0.clone();
        let response = tauri::async_runtime::spawn_blocking(move || {
            handle.run_mobile_plugin::<Access>(
                "requestAccess",
                std::collections::HashMap::<String, String>::new(),
            )
        })
        .await
        .map_err(|_| {
            adb_core::backend::PocError::new(
                "permission",
                "Could not request local network access.",
            )
        })?
        .map_err(|e| adb_core::backend::PocError::new("permission", e.to_string()))?;
        require_grant(response.granted)?;
    }
    #[cfg(not(target_os = "android"))]
    let _ = app;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovery_permission_denial_stays_distinct_from_an_empty_scan() {
        assert_eq!(discovery_error(Some("permission"), "Allow access".into()).kind, "permission");
        assert_eq!(discovery_error(None, "Scan failed".into()).kind, "discovery");
    }
    #[test]
    fn denied_access_has_recoverable_permission_result() {
        let e = require_grant(false).unwrap_err();
        assert_eq!(e.kind, "permission");
        assert!(e.message.contains("Settings"));
        assert!(require_grant(true).is_ok());
    }
}

#[cfg(target_os = "android")]
pub async fn set_appearance(app: &AppHandle, dark: bool) -> crate::Result<()> {
    let handle = app.state::<LanHandle<tauri::Wry>>().0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        handle.run_mobile_plugin::<serde_json::Value>(
            "setAppearance",
            serde_json::json!({ "dark": dark }),
        )
    })
    .await
    .map_err(|e| crate::err(e))?
    .map_err(|e| crate::err(e))?;
    Ok(())
}
