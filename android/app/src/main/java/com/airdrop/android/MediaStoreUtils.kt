package com.airdrop.android

import android.content.ContentValues
import android.content.Context
import android.media.MediaScannerConnection
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.provider.MediaStore
import android.util.Log
import java.io.File
import java.io.FileInputStream
import java.io.FileOutputStream

object MediaStoreUtils {
    private const val TAG = "MediaStoreUtils"

    fun inferMimeType(fileName: String): String {
        val lower = fileName.lowercase()
        return when {
            lower.endsWith(".jpg") || lower.endsWith(".jpeg") -> "image/jpeg"
            lower.endsWith(".png") -> "image/png"
            lower.endsWith(".heic") -> "image/heic"
            lower.endsWith(".gif") -> "image/gif"
            lower.endsWith(".webp") -> "image/webp"
            lower.endsWith(".mp4") -> "video/mp4"
            lower.endsWith(".mov") -> "video/quicktime"
            lower.endsWith(".mp3") -> "audio/mpeg"
            lower.endsWith(".m4a") -> "audio/mp4"
            lower.endsWith(".pdf") -> "application/pdf"
            lower.endsWith(".txt") -> "text/plain"
            lower.endsWith(".zip") -> "application/zip"
            else -> "application/octet-stream"
        }
    }

    /**
     * Scans and indexes a received file into Android's MediaStore so it immediately
     * appears in Gallery, Google Photos, or Downloads.
     */
    fun indexReceivedFile(context: Context, file: File): Uri? {
        if (!file.exists()) {
            Log.w(TAG, "File does not exist for indexing: ${file.absolutePath}")
            return null
        }

        val mimeType = inferMimeType(file.name)
        Log.i(TAG, "Indexing received file ${file.name} ($mimeType)")

        // 1. Run MediaScannerConnection to notify system media provider
        MediaScannerConnection.scanFile(
            context.applicationContext,
            arrayOf(file.absolutePath),
            arrayOf(mimeType)
        ) { path, uri ->
            Log.i(TAG, "MediaScanner indexed $path -> $uri")
        }

        // 2. On Android 10+ (API 29+), explicitly copy media to Scoped MediaStore collections
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            try {
                val resolver = context.contentResolver
                val contentValues = ContentValues().apply {
                    put(MediaStore.MediaColumns.DISPLAY_NAME, file.name)
                    put(MediaStore.MediaColumns.MIME_TYPE, mimeType)
                }

                val collectionUri = when {
                    mimeType.startsWith("image/") -> {
                        contentValues.put(
                            MediaStore.Images.Media.RELATIVE_PATH,
                            Environment.DIRECTORY_PICTURES + "/AirDrop"
                        )
                        MediaStore.Images.Media.EXTERNAL_CONTENT_URI
                    }
                    mimeType.startsWith("video/") -> {
                        contentValues.put(
                            MediaStore.Video.Media.RELATIVE_PATH,
                            Environment.DIRECTORY_MOVIES + "/AirDrop"
                        )
                        MediaStore.Video.Media.EXTERNAL_CONTENT_URI
                    }
                    mimeType.startsWith("audio/") -> {
                        contentValues.put(
                            MediaStore.Audio.Media.RELATIVE_PATH,
                            Environment.DIRECTORY_MUSIC + "/AirDrop"
                        )
                        MediaStore.Audio.Media.EXTERNAL_CONTENT_URI
                    }
                    else -> {
                        contentValues.put(
                            MediaStore.Downloads.RELATIVE_PATH,
                            Environment.DIRECTORY_DOWNLOADS + "/AirDrop"
                        )
                        MediaStore.Downloads.EXTERNAL_CONTENT_URI
                    }
                }

                val itemUri = resolver.insert(collectionUri, contentValues)
                if (itemUri != null) {
                    resolver.openOutputStream(itemUri)?.use { out ->
                        FileInputStream(file).use { input ->
                            input.copyTo(out)
                        }
                    }
                    Log.i(TAG, "Successfully copied to MediaStore: $itemUri")
                    return itemUri
                }
            } catch (e: Exception) {
                Log.w(TAG, "Scoped MediaStore insert failed, relying on MediaScanner: ${e.message}")
            }
        }

        return Uri.fromFile(file)
    }
}
