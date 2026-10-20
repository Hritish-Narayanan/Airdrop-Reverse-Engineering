package com.airdrop.android

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.net.wifi.WifiManager
import android.util.Log

class AirDropNsdManager(private val context: Context) {
    companion object {
        private const val TAG = "AirDropNsdManager"
        private const val SERVICE_TYPE = "_airdrop._tcp."
    }

    private val nsdManager: NsdManager =
        context.getSystemService(Context.NSD_SERVICE) as NsdManager
    private val wifiManager: WifiManager =
        context.applicationContext.getSystemService(Context.WIFI_SERVICE) as WifiManager

    private var multicastLock: WifiManager.MulticastLock? = null
    private var isRegistered = false

    private val registrationListener = object : NsdManager.RegistrationListener {
        override fun onServiceRegistered(serviceInfo: NsdServiceInfo?) {
            Log.i(TAG, "AirDrop mDNS Service registered: ${serviceInfo?.serviceName}")
            isRegistered = true
        }

        override fun onRegistrationFailed(serviceInfo: NsdServiceInfo?, errorCode: Int) {
            Log.e(TAG, "AirDrop mDNS registration failed: errorCode=$errorCode")
            isRegistered = false
        }

        override fun onServiceUnregistered(serviceInfo: NsdServiceInfo?) {
            Log.i(TAG, "AirDrop mDNS Service unregistered")
            isRegistered = false
        }

        override fun onUnregistrationFailed(serviceInfo: NsdServiceInfo?, errorCode: Int) {
            Log.e(TAG, "AirDrop mDNS unregistration failed: errorCode=$errorCode")
        }
    }

    fun registerService(deviceName: String, port: Int) {
        // Acquire multicast lock so Android doesn't filter out mDNS packets
        if (multicastLock == null) {
            multicastLock = wifiManager.createMulticastLock("AirDropMulticastLock").apply {
                setReferenceCounted(true)
                acquire()
            }
        }

        val serviceInfo = NsdServiceInfo().apply {
            serviceName = "012345${String.format("%06x", port)}"
            serviceType = SERVICE_TYPE
            this.port = port
            setAttribute("flags", "0x01")
            setAttribute("model", "Pixel")
        }

        try {
            nsdManager.registerService(serviceInfo, NsdManager.PROTOCOL_DNS_SD, registrationListener)
        } catch (e: Exception) {
            Log.e(TAG, "Error registering NSD service: ${e.message}")
        }
    }

    fun unregisterService() {
        if (isRegistered) {
            try {
                nsdManager.unregisterService(registrationListener)
            } catch (e: Exception) {
                Log.e(TAG, "Error unregistering NSD service: ${e.message}")
            }
        }
        multicastLock?.let {
            if (it.isHeld) {
                it.release()
            }
        }
        multicastLock = null
    }
}
