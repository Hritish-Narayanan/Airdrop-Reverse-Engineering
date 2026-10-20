package com.airdrop.android

import android.content.Intent
import android.os.Build
import android.service.quicksettings.Tile
import android.service.quicksettings.TileService
import androidx.annotation.RequiresApi

@RequiresApi(Build.VERSION_CODES.N)
class AirDropTileService : TileService() {

    override fun onStartListening() {
        super.onStartListening()
        updateTileState()
    }

    override fun onClick() {
        super.onClick()
        val isCurrentlyRunning = AirDropForegroundService.isRunning.value

        if (isCurrentlyRunning) {
            val stopIntent = Intent(this, AirDropForegroundService::class.java).apply {
                action = AirDropForegroundService.ACTION_STOP
            }
            startService(stopIntent)
        } else {
            val startIntent = Intent(this, AirDropForegroundService::class.java).apply {
                action = AirDropForegroundService.ACTION_START
                putExtra(AirDropForegroundService.EXTRA_DEVICE_NAME, "Nothing Phone (A001)")
                putExtra(AirDropForegroundService.EXTRA_DEVICE_MODEL, "MacBookPro18,1")
                putExtra(AirDropForegroundService.EXTRA_EVERYONE, true)
                putExtra(AirDropForegroundService.EXTRA_PORT, 8770)
            }
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                startForegroundService(startIntent)
            } else {
                startService(startIntent)
            }
        }

        // Delay slight tick then update
        qsTile?.let {
            it.state = if (!isCurrentlyRunning) Tile.STATE_ACTIVE else Tile.STATE_INACTIVE
            it.subtitle = if (!isCurrentlyRunning) "Everyone" else "Off"
            it.updateTile()
        }
    }

    private fun updateTileState() {
        val tile = qsTile ?: return
        val running = AirDropForegroundService.isRunning.value

        tile.state = if (running) Tile.STATE_ACTIVE else Tile.STATE_INACTIVE
        tile.label = "AirDrop"
        tile.subtitle = if (running) "Everyone" else "Off"
        tile.updateTile()
    }
}
