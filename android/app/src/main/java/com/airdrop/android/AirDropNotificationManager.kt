package com.airdrop.android

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import androidx.core.app.NotificationCompat

object AirDropNotificationManager {
    const val CHANNEL_ID_SERVICE = "airdrop_background_service"
    const val CHANNEL_ID_TRANSFERS = "airdrop_transfers"
    const val CHANNEL_ID_CONSENT = "airdrop_consent"
    const val NOTIFICATION_ID_SERVICE = 1001
    const val NOTIFICATION_ID_CONSENT = 1002
    const val NOTIFICATION_ID_TRANSFER_BASE = 2000

    fun createNotificationChannels(context: Context) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val manager = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager

            val serviceChannel = NotificationChannel(
                CHANNEL_ID_SERVICE,
                "AirDrop Service",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "Shows active AirDrop discoverability state"
                setShowBadge(false)
            }

            val transferChannel = NotificationChannel(
                CHANNEL_ID_TRANSFERS,
                "AirDrop Transfers",
                NotificationManager.IMPORTANCE_HIGH
            ).apply {
                description = "Notifications for incoming and completed AirDrop transfers"
                enableVibration(true)
            }

            val consentChannel = NotificationChannel(
                CHANNEL_ID_CONSENT,
                "AirDrop Transfer Requests",
                NotificationManager.IMPORTANCE_HIGH
            ).apply {
                description = "Interactive Accept/Decline prompts for incoming AirDrop requests"
                enableVibration(true)
                setShowBadge(true)
            }

            manager.createNotificationChannel(serviceChannel)
            manager.createNotificationChannel(transferChannel)
            manager.createNotificationChannel(consentChannel)
        }
    }

    fun buildForegroundNotification(
        context: Context,
        deviceName: String,
        isEveryone: Boolean
    ): Notification {
        val launchIntent = Intent(context, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_SINGLE_TOP
        }
        val pendingIntent = PendingIntent.getActivity(
            context,
            0,
            launchIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        val stopIntent = Intent(context, AirDropForegroundService::class.java).apply {
            action = AirDropForegroundService.ACTION_STOP
        }
        val stopPendingIntent = PendingIntent.getService(
            context,
            1,
            stopIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        val modeText = if (isEveryone) "Everyone" else "Contacts Only"

        return NotificationCompat.Builder(context, CHANNEL_ID_SERVICE)
            .setContentTitle("AirDrop Active")
            .setContentText("Discoverable as \"$deviceName\" ($modeText)")
            .setSmallIcon(android.R.drawable.ic_menu_share)
            .setOngoing(true)
            .setContentIntent(pendingIntent)
            .addAction(android.R.drawable.ic_menu_close_clear_cancel, "Turn Off", stopPendingIntent)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .build()
    }

    fun showTransferCompleteNotification(
        context: Context,
        fileName: String,
        itemCount: Int,
        targetUri: Uri?
    ) {
        val viewIntent = if (targetUri != null) {
            Intent(Intent.ACTION_VIEW).apply {
                setDataAndType(targetUri, context.contentResolver.getType(targetUri) ?: "*/*")
                flags = Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_ACTIVITY_NEW_TASK
            }
        } else {
            Intent(context, MainActivity::class.java)
        }

        val pendingIntent = PendingIntent.getActivity(
            context,
            (System.currentTimeMillis() % 10000).toInt(),
            viewIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        val contentText = if (itemCount > 1) {
            "Received $fileName and ${itemCount - 1} other item(s)"
        } else {
            "Received $fileName"
        }

        val notification = NotificationCompat.Builder(context, CHANNEL_ID_TRANSFERS)
            .setContentTitle("AirDrop Transfer Complete")
            .setContentText(contentText)
            .setSmallIcon(android.R.drawable.stat_sys_download_done)
            .setAutoCancel(true)
            .setContentIntent(pendingIntent)
            .setPriority(NotificationCompat.PRIORITY_HIGH)
            .build()

        val manager = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        manager.notify(NOTIFICATION_ID_TRANSFER_BASE + (System.currentTimeMillis() % 100).toInt(), notification)
    }

    fun showIncomingTransferPrompt(
        context: Context,
        transferId: String,
        senderName: String,
        fileCount: Int,
        totalBytes: Long,
        iconDataBase64: String? = null
    ) {
        val acceptIntent = Intent(context, AirDropForegroundService::class.java).apply {
            action = AirDropForegroundService.ACTION_CONSENT_DECISION
            putExtra(AirDropForegroundService.EXTRA_TRANSFER_ID, transferId)
            putExtra(AirDropForegroundService.EXTRA_CONSENT_ACCEPT, true)
        }
        val acceptPending = PendingIntent.getService(
            context,
            101,
            acceptIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        val declineIntent = Intent(context, AirDropForegroundService::class.java).apply {
            action = AirDropForegroundService.ACTION_CONSENT_DECISION
            putExtra(AirDropForegroundService.EXTRA_TRANSFER_ID, transferId)
            putExtra(AirDropForegroundService.EXTRA_CONSENT_ACCEPT, false)
        }
        val declinePending = PendingIntent.getService(
            context,
            102,
            declineIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        val sizeFormatted = formatFileSize(totalBytes)
        val filesText = if (fileCount == 1) "1 item ($sizeFormatted)" else "$fileCount items ($sizeFormatted)"

        val thumbnailBitmap = iconDataBase64?.let { b64 ->
            try {
                val decoded = android.util.Base64.decode(b64, android.util.Base64.DEFAULT)
                android.graphics.BitmapFactory.decodeByteArray(decoded, 0, decoded.size)
            } catch (e: Exception) {
                null
            }
        }

        val builder = NotificationCompat.Builder(context, CHANNEL_ID_CONSENT)
            .setContentTitle("AirDrop from \"$senderName\"")
            .setContentText("Wants to share $filesText")
            .setSmallIcon(android.R.drawable.stat_sys_download)
            .setPriority(NotificationCompat.PRIORITY_MAX)
            .setCategory(NotificationCompat.CATEGORY_CALL)
            .setAutoCancel(true)
            .setTimeoutAfter(30000)
            .addAction(android.R.drawable.ic_menu_close_clear_cancel, "Decline", declinePending)
            .addAction(android.R.drawable.ic_input_add, "Accept", acceptPending)

        thumbnailBitmap?.let {
            builder.setLargeIcon(it)
        }

        val notification = builder.build()
        val manager = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        manager.notify(NOTIFICATION_ID_CONSENT, notification)
    }

    fun dismissConsentPrompt(context: Context) {
        val manager = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        manager.cancel(NOTIFICATION_ID_CONSENT)
    }

    private fun formatFileSize(bytes: Long): String {
        if (bytes <= 0) return "0 B"
        val units = arrayOf("B", "KB", "MB", "GB", "TB")
        val digitGroups = (Math.log10(bytes.toDouble()) / Math.log10(1024.0)).toInt().coerceIn(0, units.size - 1)
        return String.format("%.1f %s", bytes / Math.pow(1024.0, digitGroups.toDouble()), units[digitGroups])
    }
}
