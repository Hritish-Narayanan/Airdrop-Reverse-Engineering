package com.airdrop.android

import android.content.Context
import android.net.wifi.WifiManager
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.util.Log
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow

data class HotspotInfo(
    val ssid: String,
    val passphrase: String?
)

object HotspotManager {
    private const val TAG = "HotspotManager"

    private var reservation: WifiManager.LocalOnlyHotspotReservation? = null
    private val _hotspotState = MutableStateFlow<HotspotInfo?>(null)
    val hotspotState = _hotspotState.asStateFlow()

    fun startHotspot(context: Context, onSuccess: (HotspotInfo) -> Unit = {}, onFailure: (Int) -> Unit = {}) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) {
            Log.w(TAG, "LocalOnlyHotspot requires Android 8.0+")
            return
        }

        val wifiManager = context.applicationContext.getSystemService(Context.WIFI_SERVICE) as? WifiManager ?: return

        try {
            wifiManager.startLocalOnlyHotspot(object : WifiManager.LocalOnlyHotspotCallback() {
                override fun onStarted(res: WifiManager.LocalOnlyHotspotReservation?) {
                    super.onStarted(res)
                    reservation = res
                    val config = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                        res?.softApConfiguration
                    } else {
                        @Suppress("DEPRECATION")
                        res?.wifiConfiguration
                    }

                    val ssid = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                        res?.softApConfiguration?.ssid ?: "AirDrop-Direct"
                    } else {
                        @Suppress("DEPRECATION")
                        res?.wifiConfiguration?.SSID ?: "AirDrop-Direct"
                    }

                    val pass = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                        res?.softApConfiguration?.passphrase
                    } else {
                        @Suppress("DEPRECATION")
                        res?.wifiConfiguration?.preSharedKey
                    }

                    val info = HotspotInfo(ssid = ssid, passphrase = pass)
                    _hotspotState.value = info
                    Log.i(TAG, "LocalOnlyHotspot active: SSID=$ssid, pass=$pass")
                    onSuccess(info)
                }

                override fun onStopped() {
                    super.onStopped()
                    reservation = null
                    _hotspotState.value = null
                    Log.i(TAG, "LocalOnlyHotspot stopped")
                }

                override fun onFailed(reason: Int) {
                    super.onFailed(reason)
                    reservation = null
                    _hotspotState.value = null
                    Log.e(TAG, "LocalOnlyHotspot failed with reason: $reason")
                    onFailure(reason)
                }
            }, Handler(Looper.getMainLooper()))
        } catch (e: SecurityException) {
            Log.e(TAG, "SecurityException starting LocalOnlyHotspot: ${e.message}")
        }
    }

    fun stopHotspot() {
        try {
            reservation?.close()
        } catch (e: Exception) {
            Log.w(TAG, "Error closing hotspot reservation: ${e.message}")
        } finally {
            reservation = null
            _hotspotState.value = null
        }
    }
}
