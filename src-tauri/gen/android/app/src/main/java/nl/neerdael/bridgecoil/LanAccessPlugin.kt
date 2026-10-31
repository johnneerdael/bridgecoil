package nl.neerdael.bridgecoil

import android.app.Activity
import android.content.Intent
import android.os.Build
import android.graphics.Color
import androidx.core.content.ContextCompat
import androidx.core.view.WindowCompat
import app.tauri.annotation.InvokeArg
import app.tauri.Logger
import app.tauri.PermissionState
import app.tauri.annotation.Command
import app.tauri.annotation.Permission
import app.tauri.annotation.PermissionCallback
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

@InvokeArg
class AppearanceArgs { var dark: Boolean = false }

@InvokeArg
class ConnectionArgs { var connected: Boolean = false }

@TauriPlugin(permissions = [Permission(strings = ["android.permission.ACCESS_LOCAL_NETWORK"], alias = "localNetwork")])
class LanAccessPlugin(private val hostActivity: Activity) : Plugin(hostActivity) {
    private var discovery: AdbDiscovery? = null
    @Command
    fun discover(invoke: Invoke) {
        hostActivity.runOnUiThread {
            if (discovery != null) { invoke.reject("A discovery scan is already running."); return@runOnUiThread }
            val scan = AdbDiscovery(hostActivity) { discovery = null }
            discovery = scan
            scan.start(invoke)
        }
    }
    @Command
    fun keepConnection(invoke: Invoke) {
        val connected = invoke.parseArgs(ConnectionArgs::class.java).connected
        val service = Intent(hostActivity, ConnectionService::class.java)
        try {
            if (connected) {
                ContextCompat.startForegroundService(hostActivity, service)
            } else {
                hostActivity.stopService(service)
            }
        } catch (e: IllegalStateException) {
            // Android refuses to start one from the background. The connection still works
            // while the app is on screen, so this isn't worth failing over.
            Logger.error("Could not keep the connection: ${e.message}")
        }
        invoke.resolve(JSObject())
    }

    @Command
    fun setAppearance(invoke: Invoke) {
        val dark = invoke.parseArgs(AppearanceArgs::class.java).dark
        hostActivity.runOnUiThread {
            val canvas = Color.parseColor(if (dark) "#101318" else "#F1F4F8")
            hostActivity.window.decorView.setBackgroundColor(canvas)
            WindowCompat.getInsetsController(hostActivity.window, hostActivity.window.decorView).apply {
                isAppearanceLightStatusBars = !dark
                isAppearanceLightNavigationBars = !dark
            }
            invoke.resolve(JSObject())
        }
    }

    @Command
    fun requestAccess(invoke: Invoke) {
        if (Build.VERSION.SDK_INT < 37 || getPermissionState("localNetwork") == PermissionState.GRANTED) {
            finishAccess(invoke)
        } else {
            requestPermissionForAlias("localNetwork", invoke, "finishAccess")
        }
    }

    @PermissionCallback
    fun finishAccess(invoke: Invoke) {
        val result = JSObject()
        result.put("granted", Build.VERSION.SDK_INT < 37 || getPermissionState("localNetwork") == PermissionState.GRANTED)
        invoke.resolve(result)
    }
}
