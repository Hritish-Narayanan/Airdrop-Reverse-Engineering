package com.airdrop.android

import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.net.wifi.WifiManager
import android.os.Build
import android.os.FileObserver
import android.os.IBinder
import android.os.PowerManager
import android.util.Log
import androidx.core.app.ServiceCompat
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import java.io.File

class AirDropForegroundService : Service() {
    companion object {
        private const val TAG = "AirDropService"
        const val ACTION_START = "com.airdrop.android.action.START"
        const val ACTION_STOP = "com.airdrop.android.action.STOP"
        const val ACTION_CONSENT_DECISION = "com.airdrop.android.action.CONSENT_DECISION"

        const val EXTRA_DEVICE_NAME = "extra_device_name"
        const val EXTRA_DEVICE_MODEL = "extra_device_model"
        const val EXTRA_PORT = "extra_port"
        const val EXTRA_EVERYONE = "extra_everyone"
        const val EXTRA_AUTO_ACCEPT = "extra_auto_accept"
        const val EXTRA_DOWNLOAD_DIR = "extra_download_dir"
        const val EXTRA_TRANSFER_ID = "extra_transfer_id"
        const val EXTRA_CONSENT_ACCEPT = "extra_consent_accept"

        private val _isRunning = MutableStateFlow(false)
        val isRunning = _isRunning.asStateFlow()

        private val _lastReceivedFile = MutableStateFlow<File?>(null)
        val lastReceivedFile = _lastReceivedFile.asStateFlow()
    }

    private val serviceScope = CoroutineScope(Dispatchers.IO + SupervisorJob())

    private lateinit var bleManager: AirDropBleManager
    private lateinit var nsdManager: AirDropNsdManager

    private var wakeLock: PowerManager.WakeLock? = null
    private var multicastLock: WifiManager.MulticastLock? = null
    private var fileObserver: FileObserver? = null

    private var downloadDir: File? = null

    override fun onCreate() {
        super.onCreate()
        Log.i(TAG, "AirDropForegroundService created")
        AirDropNotificationManager.createNotificationChannels(this)

        bleManager = AirDropBleManager(this)
        nsdManager = AirDropNsdManager(this)

        // Register incoming consent listener to display notifications
        AirDropBridge.consentListener = { req ->
            Log.i(TAG, "Incoming consent request from ${req.senderName} (${req.fileCount} files, ${req.totalBytes} bytes)")
            AirDropNotificationManager.showIncomingTransferPrompt(
                context = this,
                transferId = req.transferId,
                senderName = req.senderName,
                fileCount = req.fileCount,
                totalBytes = req.totalBytes,
                iconDataBase64 = req.iconDataBase64
            )
        }

        val powerManager = getSystemService(Context.POWER_SERVICE) as PowerManager
        wakeLock = powerManager.newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "AirDrop::TransferWakeLock")

        val wifiManager = applicationContext.getSystemService(Context.WIFI_SERVICE) as WifiManager
        multicastLock = wifiManager.createMulticastLock("AirDrop::MulticastLock").apply {
            setReferenceCounted(true)
        }
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val action = intent?.action ?: ACTION_START

        if (action == ACTION_STOP) {
            stopAirDrop()
            stopSelf()
            return START_NOT_STICKY
        }

        if (action == ACTION_CONSENT_DECISION) {
            val transferId = intent?.getStringExtra(EXTRA_TRANSFER_ID) ?: ""
            val accept = intent?.getBooleanExtra(EXTRA_CONSENT_ACCEPT, false) ?: false
            Log.i(TAG, "User consent response: transferId=$transferId, accept=$accept")
            AirDropBridge.replyConsent(transferId, accept)
            AirDropNotificationManager.dismissConsentPrompt(this)
            return START_NOT_STICKY
        }

        val deviceName = intent?.getStringExtra(EXTRA_DEVICE_NAME) ?: "Nothing Phone (A001)"
        val deviceModel = intent?.getStringExtra(EXTRA_DEVICE_MODEL) ?: "MacBookPro18,1"
        val port = intent?.getIntExtra(EXTRA_PORT, 8770) ?: 8770
        val isEveryone = intent?.getBooleanExtra(EXTRA_EVERYONE, true) ?: true
        val autoAccept = intent?.getBooleanExtra(EXTRA_AUTO_ACCEPT, true) ?: true
        val dirPath = intent?.getStringExtra(EXTRA_DOWNLOAD_DIR)
            ?: getExternalFilesDir(null)?.absolutePath
            ?: cacheDir.absolutePath

        downloadDir = File(dirPath).apply { mkdirs() }

        startAirDrop(deviceName, deviceModel, port, isEveryone, autoAccept, downloadDir!!)
        return START_STICKY
    }

    private fun startAirDrop(
        deviceName: String,
        deviceModel: String,
        port: Int,
        isEveryone: Boolean,
        autoAccept: Boolean,
        targetDir: File
    ) {
        Log.i(TAG, "Starting AirDrop service as '$deviceName' ($deviceModel) on port $port (autoAccept=$autoAccept)")

        // 1. Acquire Locks
        try {
            if (wakeLock?.isHeld != true) {
                wakeLock?.acquire(24 * 60 * 60 * 1000L) // up to 24h or until stopped
            }
            if (multicastLock?.isHeld != true) {
                multicastLock?.acquire()
            }
        } catch (e: Exception) {
            Log.w(TAG, "Could not acquire system locks: ${e.message}")
        }

        // 2. Start Foreground Notification
        val notification = AirDropNotificationManager.buildForegroundNotification(
            this,
            deviceName,
            isEveryone
        )

        val foregroundType = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE
        } else {
            0
        }

        ServiceCompat.startForeground(
            this,
            AirDropNotificationManager.NOTIFICATION_ID_SERVICE,
            notification,
            foregroundType
        )

        // 3. Start Rust Receiver Server
        AirDropBridge.startServer(
            port = port,
            downloadDir = targetDir.absolutePath,
            deviceName = deviceName,
            autoAccept = autoAccept
        )

        // 4. Register mDNS Bonjour with Apple Model Persona
        nsdManager.registerService(
            deviceName = deviceName,
            port = port,
            isEveryone = isEveryone,
            deviceModel = deviceModel
        )

        // 5. Start Dual BLE Advertising
        bleManager.startAdvertising(everyoneMode = isEveryone)

        // 6. Monitor download directory for newly received files
        startMonitoringDirectory(targetDir)

        _isRunning.value = true
    }

    private fun startMonitoringDirectory(dir: File) {
        fileObserver?.stopWatching()

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            fileObserver = object : FileObserver(dir, CLOSE_WRITE or MOVED_TO) {
                override fun onEvent(event: Int, path: String?) {
                    if (path != null) {
                        handleNewReceivedFile(File(dir, path))
                    }
                }
            }
        } else {
            @Suppress("DEPRECATION")
            fileObserver = object : FileObserver(dir.absolutePath, CLOSE_WRITE or MOVED_TO) {
                override fun onEvent(event: Int, path: String?) {
                    if (path != null) {
                        handleNewReceivedFile(File(dir, path))
                    }
                }
            }
        }
        fileObserver?.startWatching()
    }

    private fun handleNewReceivedFile(file: File) {
        if (!file.exists() || file.isDirectory) return

        serviceScope.launch {
            // Small debounce to ensure file writes are completely flushed
            delay(300)
            if (file.exists() && file.length() > 0) {
                Log.i(TAG, "New file completed: ${file.name} (${file.length()} bytes)")
                val mediaUri = MediaStoreUtils.indexReceivedFile(this@AirDropForegroundService, file)
                AirDropNotificationManager.showTransferCompleteNotification(
                    this@AirDropForegroundService,
                    file.name,
                    1,
                    mediaUri
                )
                _lastReceivedFile.value = file
            }
        }
    }

    private fun stopAirDrop() {
        Log.i(TAG, "Stopping AirDrop service")
        AirDropNotificationManager.dismissConsentPrompt(this)
        fileObserver?.stopWatching()
        fileObserver = null

        bleManager.stopAdvertising()
        nsdManager.unregisterService()
        AirDropBridge.stopServer()

        try {
            if (multicastLock?.isHeld == true) {
                multicastLock?.release()
            }
            if (wakeLock?.isHeld == true) {
                wakeLock?.release()
            }
        } catch (e: Exception) {
            Log.w(TAG, "Error releasing system locks: ${e.message}")
        }

        _isRunning.value = false
        stopForeground(STOP_FOREGROUND_REMOVE)
    }

    override fun onDestroy() {
        stopAirDrop()
        serviceScope.cancel()
        super.onDestroy()
    }

    override fun onBind(intent: Intent?): IBinder? = null
}
