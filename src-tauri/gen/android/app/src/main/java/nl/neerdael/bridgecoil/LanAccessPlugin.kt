package nl.neerdael.bridgecoil

import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.ContentValues
import android.content.Intent
import android.os.Build
import android.os.Environment
import android.graphics.Color
import android.net.Uri
import android.provider.MediaStore
import androidx.annotation.RequiresApi
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
import java.io.File

@InvokeArg
class AppearanceArgs { var dark: Boolean = false }

@InvokeArg
class ConnectionArgs { var connected: Boolean = false }

@InvokeArg
class DownloadArgs {
    var path: String = ""
    var name: String = ""
    var mime: String = ""
}

@InvokeArg
class OpenArgs {
    var uri: String = ""
    var mime: String = ""
}

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

    /** Resolves with no location on Android 9 and older, which have no Downloads collection. */
    @Command
    fun saveToDownloads(invoke: Invoke) {
        val args = invoke.parseArgs(DownloadArgs::class.java)
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.Q) {
            invoke.resolve(JSObject())
        } else {
            Thread { copyToDownloads(invoke, args) }.start()
        }
    }

    @RequiresApi(Build.VERSION_CODES.Q)
    private fun copyToDownloads(invoke: Invoke, args: DownloadArgs) {
        val resolver = hostActivity.contentResolver
        val values = ContentValues().apply {
            put(MediaStore.MediaColumns.DISPLAY_NAME, args.name)
            put(MediaStore.MediaColumns.MIME_TYPE, args.mime)
            put(MediaStore.MediaColumns.RELATIVE_PATH, Environment.DIRECTORY_DOWNLOADS)
            put(MediaStore.MediaColumns.IS_PENDING, 1)
        }
        val collection = MediaStore.Downloads.getContentUri(MediaStore.VOLUME_EXTERNAL_PRIMARY)
        val item = resolver.insert(collection, values)
        if (item == null) {
            invoke.reject("Android couldn't create the file in Download.")
            return
        }
        try {
            resolver.openOutputStream(item).use { target ->
                File(args.path).inputStream().use { it.copyTo(target!!) }
            }
            resolver.update(
                item,
                ContentValues().apply { put(MediaStore.MediaColumns.IS_PENDING, 0) },
                null,
                null,
            )
            val result = JSObject()
            result.put("location", Environment.DIRECTORY_DOWNLOADS)
            result.put("uri", item.toString())
            invoke.resolve(result)
        } catch (e: Exception) {
            resolver.delete(item, null, null)
            invoke.reject("The file couldn't be saved to Download: ${e.message}")
        }
    }

    /** Opens a saved capture in whichever app the phone uses for that kind of file. */
    @Command
    fun openCapture(invoke: Invoke) {
        val args = invoke.parseArgs(OpenArgs::class.java)
        val viewer = Intent(Intent.ACTION_VIEW).apply {
            setDataAndType(Uri.parse(args.uri), args.mime)
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        }
        try {
            hostActivity.startActivity(viewer)
            invoke.resolve(JSObject())
        } catch (e: ActivityNotFoundException) {
            invoke.reject("No app on this phone can open this file.")
        }
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
