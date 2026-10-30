package nl.neerdael.adbstudio.poc

import android.app.Activity
import android.os.Build
import app.tauri.PermissionState
import app.tauri.annotation.Command
import app.tauri.annotation.Permission
import app.tauri.annotation.PermissionCallback
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

@TauriPlugin(permissions = [Permission(strings = ["android.permission.ACCESS_LOCAL_NETWORK"], alias = "localNetwork")])
class LanAccessPlugin(activity: Activity) : Plugin(activity) {
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
