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

data class IncomingConsentRequest(
    val transferId: String,
    val senderName: String,
    val senderModel: String,
    val fileCount: Int,
    val totalBytes: Long,
    val iconDataBase64: String? = null
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

    // Reactive State for Incoming Transfer Consent
    val incomingConsent = kotlinx.coroutines.flow.MutableStateFlow<IncomingConsentRequest?>(null)
    var consentListener: ((IncomingConsentRequest) -> Unit)? = null

    @JvmStatic
    fun onIncomingConsent(
        transferId: String,
        senderName: String,
        senderModel: String,
        fileCount: Int,
        totalBytes: Long,
        iconDataBase64: String? = null
    ) {
        val req = IncomingConsentRequest(
            transferId = transferId,
            senderName = senderName,
            senderModel = senderModel,
            fileCount = fileCount,
            totalBytes = totalBytes,
            iconDataBase64 = iconDataBase64
        )
        incomingConsent.value = req
        consentListener?.invoke(req)
    }

    // Native C-ABI Functions
    @JvmStatic private external fun init()
    @JvmStatic private external fun generateBlePayload(everyone: Boolean): ByteArray
    @JvmStatic private external fun generateNearbyActionPayload(flags: Int): ByteArray
    @JvmStatic private external fun startReceiver(port: Int, downloadDir: String, name: String, autoAccept: Boolean): Boolean
    @JvmStatic private external fun stopReceiver()
    @JvmStatic private external fun respondToConsent(transferId: String, accept: Boolean): Boolean
    @JvmStatic private external fun sendFiles(targetUrl: String, filePaths: Array<String>, senderName: String, senderModel: String): String
    @JvmStatic private external fun scanPeers(timeoutSecs: Int): String

    fun getBlePayload(everyone: Boolean = true): ByteArray {
        return generateBlePayload(everyone)
    }

    fun getNearbyActionPayload(flags: Int = 1): ByteArray {
        return generateNearbyActionPayload(flags)
    }

    fun startServer(port: Int, downloadDir: String, deviceName: String, autoAccept: Boolean = true): Boolean {
        return startReceiver(port, downloadDir, deviceName, autoAccept)
    }

    fun stopServer() {
        stopReceiver()
    }

    fun replyConsent(transferId: String, accept: Boolean): Boolean {
        if (incomingConsent.value?.transferId == transferId) {
            incomingConsent.value = null
        }
        return respondToConsent(transferId, accept)
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
