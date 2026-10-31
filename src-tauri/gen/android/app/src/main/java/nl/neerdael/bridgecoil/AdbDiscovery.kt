package nl.neerdael.bridgecoil

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.os.Build
import android.os.Handler
import android.os.Looper
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import org.json.JSONArray
import java.util.ArrayDeque

/** One bounded DNS-SD scan. No ADB sockets or automatic authorization. */
class AdbDiscovery(context: Context, private val onFinish: () -> Unit) {
    private val manager = context.getSystemService(Context.NSD_SERVICE) as NsdManager
    private val main = Handler(Looper.getMainLooper())
    private val listeners = mutableMapOf<String, NsdManager.DiscoveryListener>()
    private val pending = ArrayDeque<Pair<NsdServiceInfo, String>>()
    private val live = mutableSetOf<String>()
    private val rows = linkedMapOf<String, Pair<String, JSObject>>()
    private val warnings = linkedSetOf<String>()
    private var resolver: NsdManager.ResolveListener? = null
    private var invoke: Invoke? = null
    private var active = false
    private val deadline = Runnable { finish() }
    private fun key(info: NsdServiceInfo, type: String) = "$type/${info.serviceName}"

    fun start(call: Invoke) {
        invoke = call
        active = true
        main.postDelayed(deadline, 4000)
        for (type in listOf("_adb-tls-connect._tcp.", "_adb-tls-pairing._tcp.", "_adb._tcp.")) {
            val listener = object : NsdManager.DiscoveryListener {
                override fun onDiscoveryStarted(serviceType: String) {}
                override fun onDiscoveryStopped(serviceType: String) {}
                override fun onStopDiscoveryFailed(serviceType: String, errorCode: Int) {}
                override fun onStartDiscoveryFailed(serviceType: String, errorCode: Int) {
                    main.post { if (active) warnings.add("Could not browse $type (Android code $errorCode).") }
                }
                override fun onServiceFound(info: NsdServiceInfo) {
                    main.post {
                        if (!active || live.size >= 64) return@post
                        if (live.add(key(info, type))) { pending.add(info to type); resolveNext() }
                    }
                }
                override fun onServiceLost(info: NsdServiceInfo) {
                    main.post {
                        val id = key(info, type)
                        live.remove(id)
                        rows.entries.removeAll { it.value.first == id }
                    }
                }
            }
            listeners[type] = listener
            try { manager.discoverServices(type, NsdManager.PROTOCOL_DNS_SD, listener) }
            catch (e: SecurityException) { fail("Local network access was denied. Allow Bridgecoil in Android app permissions.", "permission"); return }
            catch (e: Exception) { warnings.add("Could not browse $type.") }
        }
    }

    @Suppress("DEPRECATION")
    private fun resolveNext() {
        if (!active || resolver != null) return
        val (info, type) = pending.poll() ?: return
        val id = key(info, type)
        if (!live.contains(id)) { resolveNext(); return }
        val listener = object : NsdManager.ResolveListener {
            override fun onResolveFailed(serviceInfo: NsdServiceInfo, errorCode: Int) {
                main.post {
                    if (!active) return@post
                    warnings.add("Some ADB addresses could not be resolved. Try Find again.")
                    resolver = null
                    resolveNext()
                }
            }
            override fun onServiceResolved(serviceInfo: NsdServiceInfo) {
                main.post {
                    if (!active) return@post
                    if (live.contains(id) && serviceInfo.port in 1..65535) {
                        val hosts = if (Build.VERSION.SDK_INT >= 34) serviceInfo.hostAddresses else listOfNotNull(serviceInfo.host)
                        for (address in hosts.sortedBy { if (it is java.net.Inet4Address) 0 else 1 }.take(4)) {
                            val host = address.hostAddress ?: continue
                            if (rows.size >= 128) break
                            val row = JSObject()
                            row.put("name", info.serviceName.take(160))
                            row.put("serviceType", type)
                            row.put("host", host)
                            row.put("port", serviceInfo.port)
                            rows["$type/$host/${serviceInfo.port}"] = id to row
                        }
                    }
                    resolver = null
                    resolveNext()
                }
            }
        }
        resolver = listener
        try { manager.resolveService(info, listener) }
        catch (e: SecurityException) { fail("Local network access was denied. Allow Bridgecoil in Android app permissions.", "permission") }
        catch (e: Exception) { resolver = null; warnings.add("Some ADB addresses could not be resolved."); resolveNext() }
    }

    private fun cleanup() {
        active = false
        main.removeCallbacks(deadline)
        for (listener in listeners.values) {
            try { manager.stopServiceDiscovery(listener) } catch (_: Exception) {}
        }
        if (Build.VERSION.SDK_INT >= 34) resolver?.let {
            try { manager.stopServiceResolution(it) } catch (_: Exception) {}
        }
        listeners.clear()
        pending.clear()
        resolver = null
        onFinish()
    }
    private fun fail(message: String, code: String? = null) {
        val call = invoke ?: return
        invoke = null
        cleanup()
        call.reject(message, code)
    }
    private fun finish() {
        val call = invoke ?: return
        invoke = null
        val result = JSObject()
        result.put("services", JSONArray(rows.values.map { it.second }))
        result.put("warnings", JSONArray(warnings.take(3)))
        cleanup()
        call.resolve(result)
    }
}
