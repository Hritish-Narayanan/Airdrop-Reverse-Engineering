package com.airdrop.android

import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothManager
import android.bluetooth.le.AdvertiseCallback
import android.bluetooth.le.AdvertiseData
import android.bluetooth.le.AdvertiseSettings
import android.bluetooth.le.BluetoothLeAdvertiser
import android.content.Context
import android.util.Log

class AirDropBleManager(private val context: Context) {
    companion object {
        private const val TAG = "AirDropBleManager"
        private const val APPLE_MANUFACTURER_ID = 0x004C
    }

    private val bluetoothManager: BluetoothManager? =
        context.getSystemService(Context.BLUETOOTH_SERVICE) as? BluetoothManager
    private val bluetoothAdapter: BluetoothAdapter? = bluetoothManager?.adapter
    private var advertiser: BluetoothLeAdvertiser? = null
    private var isAdvertising = false

    private val advertiseCallback = object : AdvertiseCallback() {
        override fun onStartSuccess(settingsInEffect: AdvertiseSettings?) {
            super.onStartSuccess(settingsInEffect)
            Log.i(TAG, "AirDrop BLE primary advertisement started successfully")
            isAdvertising = true
        }

        override fun onStartFailure(errorCode: Int) {
            super.onStartFailure(errorCode)
            Log.e(TAG, "AirDrop BLE primary advertisement failed with error code: $errorCode")
            isAdvertising = false
        }
    }

    private val nearbyActionCallback = object : AdvertiseCallback() {
        override fun onStartSuccess(settingsInEffect: AdvertiseSettings?) {
            super.onStartSuccess(settingsInEffect)
            Log.i(TAG, "AirDrop NearbyAction (Type 0x10) BLE advertisement started successfully")
        }

        override fun onStartFailure(errorCode: Int) {
            super.onStartFailure(errorCode)
            Log.w(TAG, "AirDrop NearbyAction BLE advertisement failed with error code: $errorCode")
        }
    }

    fun startAdvertising(everyoneMode: Boolean = true) {
        if (bluetoothAdapter == null || !bluetoothAdapter.isEnabled) {
            Log.w(TAG, "Bluetooth is disabled or not available")
            return
        }

        advertiser = bluetoothAdapter.bluetoothLeAdvertiser
        if (advertiser == null) {
            Log.e(TAG, "BLE advertising is not supported on this device")
            return
        }

        // 1. Primary AirDrop Type 0x05 Payload
        val blePayload = AirDropBridge.getBlePayload(everyone = everyoneMode)
        val settings = AdvertiseSettings.Builder()
            .setAdvertiseMode(AdvertiseSettings.ADVERTISE_MODE_LOW_LATENCY)
            .setTxPowerLevel(AdvertiseSettings.ADVERTISE_TX_POWER_HIGH)
            .setConnectable(false)
            .setTimeout(0) // Advertise indefinitely until stopped
            .build()

        val data = AdvertiseData.Builder()
            .setIncludeDeviceName(false)
            .setIncludeTxPowerLevel(false)
            .addManufacturerData(APPLE_MANUFACTURER_ID, blePayload)
            .build()

        // 2. Secondary NearbyAction Type 0x10 Payload (AirDrop Proximity beacon)
        val nearbyActionPayload = AirDropBridge.getNearbyActionPayload(flags = 0x01)
        val nearbyData = AdvertiseData.Builder()
            .setIncludeDeviceName(false)
            .setIncludeTxPowerLevel(false)
            .addManufacturerData(APPLE_MANUFACTURER_ID, nearbyActionPayload)
            .build()

        try {
            advertiser?.startAdvertising(settings, data, advertiseCallback)
            advertiser?.startAdvertising(settings, nearbyData, nearbyActionCallback)
        } catch (e: SecurityException) {
            Log.e(TAG, "Missing BLUETOOTH_ADVERTISE permission: ${e.message}")
        }
    }

    fun stopAdvertising() {
        if (isAdvertising && advertiser != null) {
            try {
                advertiser?.stopAdvertising(advertiseCallback)
                advertiser?.stopAdvertising(nearbyActionCallback)
                Log.i(TAG, "AirDrop BLE advertisements stopped")
            } catch (e: SecurityException) {
                Log.e(TAG, "SecurityException while stopping advertising: ${e.message}")
            } finally {
                isAdvertising = false
            }
        }
    }
}
