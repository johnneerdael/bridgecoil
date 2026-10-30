//! Android 17 runtime LAN permission, requested only when the user starts pairing/connecting.
#[cfg(target_os = "android")]
use tauri::Manager;
use tauri::{
    AppHandle, Runtime,
    plugin::{Builder, TauriPlugin},
};
#[cfg(target_os = "android")]
struct LanHandle<R: Runtime>(tauri::plugin::PluginHandle<R>);

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("local-network")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            {
                let handle =
                    api.register_android_plugin("nl.neerdael.adbstudio.poc", "LanAccessPlugin")?;
                app.manage(LanHandle(handle));
            }
            #[cfg(not(target_os = "android"))]
            let _ = (app, api);
            Ok(())
        })
        .build()
}
#[cfg(any(target_os = "android", test))]
fn require_grant(granted: bool) -> crate::Result<()> {
    if granted {
        Ok(())
    } else {
        Err(adb_core::backend::PocError::new(
            "permission",
            "Allow local network access in Android Settings → Apps → ADB Studio → Permissions, then retry.",
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
    fn denied_access_has_recoverable_permission_result() {
        let e = require_grant(false).unwrap_err();
        assert_eq!(e.kind, "permission");
        assert!(e.message.contains("Settings"));
        assert!(require_grant(true).is_ok());
    }
}
