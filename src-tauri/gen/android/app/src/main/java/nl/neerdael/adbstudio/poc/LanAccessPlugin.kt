package nl.neerdael.adbstudio.poc

import android.app.Activity
import android.os.Build
import android.graphics.Color
import androidx.core.view.WindowCompat
import app.tauri.annotation.InvokeArg
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

@TauriPlugin(permissions = [Permission(strings = ["android.permission.ACCESS_LOCAL_NETWORK"], alias = "localNetwork")])
class LanAccessPlugin(private val hostActivity: Activity) : Plugin(hostActivity) {
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
