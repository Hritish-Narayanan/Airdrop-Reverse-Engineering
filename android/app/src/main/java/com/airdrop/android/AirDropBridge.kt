package com.airdrop.android

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject

data class DiscoveredAppleDevice(
    val instanceName: String,
    val hostName: String,
    val port: Int,
    val targetUrl: String,
    val ipAddresses: List<String>
)

data class TransferResult(
    val success: Boolean,
    val error: String?
)

object AirDropBridge {
    init {
        try {
            System.loadLibrary("airdrop_jni")
            init()
        } catch (e: UnsatisfiedLinkError) {
            System.err.println("Failed to load airdrop_jni native library: ${e.message}")
        }
    }

    // Native C-ABI Functions
    private external fun init()
    private external fun generateBlePayload(everyone: Boolean): ByteArray
    private external fun startReceiver(port: Int, downloadDir: String, name: String, autoAccept: Boolean): Boolean
    private external fun stopReceiver()
    private external fun sendFiles(targetUrl: String, filePaths: Array<String>, senderName: String, senderModel: String): String
    private external fun scanPeers(timeoutSecs: Int): String

    fun getBlePayload(everyone: Boolean = true): ByteArray {
        return generateBlePayload(everyone)
    }

    fun startServer(port: Int, downloadDir: String, deviceName: String, autoAccept: Boolean = true): Boolean {
        return startReceiver(port, downloadDir, deviceName, autoAccept)
    }

    fun stopServer() {
        stopReceiver()
    }

    suspend fun scanForAppleDevices(timeoutSecs: Int = 4): List<DiscoveredAppleDevice> = withContext(Dispatchers.IO) {
        val jsonStr = scanPeers(timeoutSecs)
        val list = mutableListOf<DiscoveredAppleDevice>()
        try {
            val jsonArray = JSONArray(jsonStr)
            for (i in 0 until jsonArray.length()) {
                val obj = jsonArray.getJSONObject(i)
                val ips = mutableListOf<String>()
                val ipArr = obj.getJSONArray("ip_addresses")
                for (j in 0 until ipArr.length()) {
                    ips.add(ipArr.getString(j))
                }
                list.add(
                    DiscoveredAppleDevice(
                        instanceName = obj.getString("instance_name"),
                        hostName = obj.getString("host_name"),
                        port = obj.getInt("port"),
                        targetUrl = obj.getString("target_url"),
                        ipAddresses = ips
                    )
                )
            }
        } catch (e: Exception) {
            e.printStackTrace()
        }
        list
    }

    suspend fun sendFilesToDevice(
        targetUrl: String,
        files: List<String>,
        senderName: String = "Android Phone",
        senderModel: String = "Pixel"
    ): TransferResult = withContext(Dispatchers.IO) {
        val jsonStr = sendFiles(targetUrl, files.toTypedArray(), senderName, senderModel)
        try {
            val obj = JSONObject(jsonStr)
            TransferResult(
                success = obj.getBoolean("success"),
                error = if (obj.has("error") && !obj.isNull("error")) obj.getString("error") else null
            )
        } catch (e: Exception) {
            TransferResult(success = false, error = e.message)
        }
    }
}
