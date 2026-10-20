package com.airdrop.android

import android.Manifest
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.os.Environment
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.content.ContextCompat
import kotlinx.coroutines.launch
import java.io.File
import java.io.FileOutputStream

import android.content.Intent

class MainActivity : ComponentActivity() {

    private lateinit var bleManager: AirDropBleManager
    private lateinit var nsdManager: AirDropNsdManager
    private var sharedFilesState = mutableStateOf<List<File>>(emptyList())

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        bleManager = AirDropBleManager(this)
        nsdManager = AirDropNsdManager(this)

        requestRequiredPermissions()

        val initialShared = extractSharedFiles(intent)
        sharedFilesState.value = initialShared

        setContent {
            AirDropAppTheme {
                AirDropScreen(
                    bleManager = bleManager,
                    nsdManager = nsdManager,
                    sharedFilesState = sharedFilesState
                )
            }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        val incoming = extractSharedFiles(intent)
        if (incoming.isNotEmpty()) {
            sharedFilesState.value = incoming
        }
    }

    private fun extractSharedFiles(intent: Intent?): List<File> {
        if (intent == null) return emptyList()
        val files = mutableListOf<File>()
        when (intent.action) {
            Intent.ACTION_SEND -> {
                val uri = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                    intent.getParcelableExtra(Intent.EXTRA_STREAM, Uri::class.java)
                } else {
                    @Suppress("DEPRECATION")
                    intent.getParcelableExtra<Uri>(Intent.EXTRA_STREAM)
                }
                uri?.let { copyUriToCache(this, it)?.let { f -> files.add(f) } }
            }
            Intent.ACTION_SEND_MULTIPLE -> {
                val uris = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                    intent.getParcelableArrayListExtra(Intent.EXTRA_STREAM, Uri::class.java)
                } else {
                    @Suppress("DEPRECATION")
                    intent.getParcelableArrayListExtra<Uri>(Intent.EXTRA_STREAM)
                }
                uris?.forEach { uri ->
                    copyUriToCache(this, uri)?.let { f -> files.add(f) }
                }
            }
        }
        return files
    }

    private fun requestRequiredPermissions() {
        val permissions = mutableListOf(
            Manifest.permission.INTERNET,
            Manifest.permission.ACCESS_WIFI_STATE,
            Manifest.permission.CHANGE_WIFI_MULTICAST_STATE
        )

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            permissions.add(Manifest.permission.BLUETOOTH_ADVERTISE)
            permissions.add(Manifest.permission.BLUETOOTH_CONNECT)
            permissions.add(Manifest.permission.BLUETOOTH_SCAN)
        } else {
            permissions.add(Manifest.permission.BLUETOOTH)
            permissions.add(Manifest.permission.BLUETOOTH_ADMIN)
            permissions.add(Manifest.permission.ACCESS_FINE_LOCATION)
        }

        val missing = permissions.filter {
            ContextCompat.checkSelfPermission(this, it) != PackageManager.PERMISSION_GRANTED
        }

        if (missing.isNotEmpty()) {
            requestPermissions(missing.toTypedArray(), 101)
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        bleManager.stopAdvertising()
        nsdManager.unregisterService()
        AirDropBridge.stopServer()
    }
}

@Composable
fun AirDropAppTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = darkColorScheme(
            primary = Color(0xFF007AFF),
            secondary = Color(0xFF5856D6),
            background = Color(0xFF121212),
            surface = Color(0xFF1E1E1E),
            onPrimary = Color.White,
            onBackground = Color(0xFFEEEEEE),
            onSurface = Color(0xFFDDDDDD)
        ),
        content = content
    )
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AirDropScreen(
    bleManager: AirDropBleManager,
    nsdManager: AirDropNsdManager,
    sharedFilesState: MutableState<List<File>>
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()

    var isReceiving by remember { mutableStateOf(false) }
    var autoAcceptTransfers by remember { mutableStateOf(false) }
    var deviceName by remember { mutableStateOf("Android Pixel") }
    var selectedFiles by sharedFilesState
    var discoveredDevices by remember { mutableStateOf<List<DiscoveredAppleDevice>>(emptyList()) }
    var isScanning by remember { mutableStateOf(false) }
    var isSending by remember { mutableStateOf(false) }
    var statusMessage by remember { mutableStateOf("Ready to send or receive AirDrop files.") }

    var directTargetUrl by remember { mutableStateOf("192.168.29.18:8780") }

    val filePickerLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.OpenMultipleDocuments()
    ) { uris: List<Uri> ->
        val tempFiles = uris.mapNotNull { uri ->
            copyUriToCache(context, uri)
        }
        selectedFiles = tempFiles
        statusMessage = "Selected ${tempFiles.size} file(s) for AirDrop."
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Icon(
                            imageVector = Icons.Default.Share,
                            contentDescription = "AirDrop",
                            tint = MaterialTheme.colorScheme.primary,
                            modifier = Modifier.size(28.dp)
                        )
                        Spacer(modifier = Modifier.width(8.dp))
                        Text("AirDrop for Android", fontWeight = FontWeight.Bold)
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.surface
                )
            )
        }
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(16.dp)
        ) {
            // Receiver Card
            Card(
                shape = RoundedCornerShape(12.dp),
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                modifier = Modifier.fillMaxWidth()
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Column(modifier = Modifier.weight(1f)) {
                            Text("AirDrop Receiver Daemon", fontWeight = FontWeight.SemiBold, fontSize = 17.sp)
                            Text(
                                if (isReceiving) "Discoverable to Apple devices" else "Receiver is inactive",
                                color = if (isReceiving) Color(0xFF34C759) else Color.Gray,
                                fontSize = 13.sp
                            )
                        }
                        Switch(
                            checked = isReceiving,
                            onCheckedChange = { checked ->
                                isReceiving = checked
                                val port = 8770
                                val downloadDir = File(context.getExternalFilesDir(Environment.DIRECTORY_DOWNLOADS), "AirDrop").apply { mkdirs() }

                                if (checked) {
                                    AirDropBridge.startServer(port, downloadDir.absolutePath, deviceName, autoAccept = autoAcceptTransfers)
                                    bleManager.startAdvertising(everyoneMode = true)
                                    nsdManager.registerService(deviceName, port)
                                    statusMessage = "Receiving active on port $port. Advertising to Apple devices."
                                } else {
                                    bleManager.stopAdvertising()
                                    nsdManager.unregisterService()
                                    AirDropBridge.stopServer()
                                    statusMessage = "AirDrop receiver stopped."
                                }
                            }
                        )
                    }

                    Spacer(modifier = Modifier.height(12.dp))
                    Divider(color = Color(0xFF333333))
                    Spacer(modifier = Modifier.height(8.dp))

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Column(modifier = Modifier.weight(1f)) {
                            Text("Auto-Accept Transfers", fontWeight = FontWeight.Medium, fontSize = 14.sp)
                            Text(
                                if (autoAcceptTransfers) "Incoming files accepted without confirmation" else "Explicit user approval required for each transfer",
                                color = Color.Gray,
                                fontSize = 12.sp
                            )
                        }
                        Switch(
                            checked = autoAcceptTransfers,
                            onCheckedChange = { autoAcceptTransfers = it }
                        )
                    }
                }
            }

            Spacer(modifier = Modifier.height(16.dp))

            // File Selection Card
            Card(
                shape = RoundedCornerShape(12.dp),
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                modifier = Modifier.fillMaxWidth()
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Text("Selected Files to Send", fontWeight = FontWeight.SemiBold, fontSize = 17.sp)
                    Spacer(modifier = Modifier.height(8.dp))

                    if (selectedFiles.isEmpty()) {
                        Text("No files selected.", color = Color.Gray, fontSize = 14.sp)
                    } else {
                        selectedFiles.forEach { file ->
                            Text("• ${file.name} (${file.length() / 1024} KB)", fontSize = 14.sp)
                        }
                    }

                    Spacer(modifier = Modifier.height(12.dp))
                    Button(
                        onClick = { filePickerLauncher.launch(arrayOf("*/*")) },
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Icon(imageVector = Icons.Default.Add, contentDescription = null)
                        Spacer(modifier = Modifier.width(8.dp))
                        Text("Select Files (Photos / Documents / Videos)")
                    }
                }
            }

            Spacer(modifier = Modifier.height(16.dp))

            // Nearby Apple Devices Card
            Card(
                shape = RoundedCornerShape(12.dp),
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                modifier = Modifier
                    .fillMaxWidth()
                    .weight(1f)
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text("Nearby Apple Devices", fontWeight = FontWeight.SemiBold, fontSize = 17.sp)
                        Button(
                            onClick = {
                                scope.launch {
                                    isScanning = true
                                    statusMessage = "Scanning for nearby AirDrop devices..."
                                    val wm = context.applicationContext.getSystemService(android.content.Context.WIFI_SERVICE) as? android.net.wifi.WifiManager
                                    val lock = wm?.createMulticastLock("AirDropScanLock")?.apply {
                                        setReferenceCounted(false)
                                        acquire()
                                    }
                                    try {
                                        discoveredDevices = AirDropBridge.scanForAppleDevices(timeoutSecs = 5)
                                    } finally {
                                        lock?.let { if (it.isHeld) it.release() }
                                    }
                                    isScanning = false
                                    statusMessage = "Found ${discoveredDevices.size} Apple device(s)."
                                }
                            },
                            enabled = !isScanning
                        ) {
                            if (isScanning) {
                                CircularProgressIndicator(modifier = Modifier.size(16.dp), color = Color.White, strokeWidth = 2.dp)
                            } else {
                                Icon(imageVector = Icons.Default.Refresh, contentDescription = null)
                                Spacer(modifier = Modifier.width(4.dp))
                                Text("Scan")
                            }
                        }
                    }

                    Spacer(modifier = Modifier.height(8.dp))

                    // Direct AirDrop Target Input Row
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        OutlinedTextField(
                            value = directTargetUrl,
                            onValueChange = { directTargetUrl = it },
                            label = { Text("Target IP:Port (e.g. Mac)") },
                            singleLine = true,
                            modifier = Modifier.weight(1f),
                            textStyle = androidx.compose.ui.text.TextStyle(fontSize = 13.sp)
                        )
                        Spacer(modifier = Modifier.width(8.dp))
                        Button(
                            onClick = {
                                scope.launch {
                                    isSending = true
                                    val target = if (directTargetUrl.startsWith("http://") || directTargetUrl.startsWith("https://")) {
                                        directTargetUrl
                                    } else {
                                        "https://$directTargetUrl"
                                    }
                                    statusMessage = "AirDropping ${selectedFiles.size} file(s) to $target..."
                                    val paths = selectedFiles.map { it.absolutePath }
                                    val res = AirDropBridge.sendFilesToDevice(
                                        targetUrl = target,
                                        files = paths,
                                        senderName = deviceName,
                                        senderModel = "Pixel"
                                    )
                                    isSending = false
                                    if (res.success) {
                                        statusMessage = "✅ Successfully AirDropped to $target!"
                                        Toast.makeText(context, "AirDrop Successful!", Toast.LENGTH_SHORT).show()
                                    } else {
                                        statusMessage = "❌ AirDrop failed: ${res.error}"
                                        Toast.makeText(context, "Failed: ${res.error}", Toast.LENGTH_LONG).show()
                                    }
                                }
                            },
                            enabled = selectedFiles.isNotEmpty() && directTargetUrl.isNotBlank() && !isSending
                        ) {
                            Text("Send")
                        }
                    }

                    Spacer(modifier = Modifier.height(8.dp))

                    if (discoveredDevices.isEmpty()) {
                        Box(
                            modifier = Modifier
                                .fillMaxWidth()
                                .weight(1f),
                            contentAlignment = Alignment.Center
                        ) {
                            Text(
                                "No Apple devices found.\nMake sure target Mac/iPhone/iPad has AirDrop set to 'Everyone'.",
                                color = Color.Gray,
                                fontSize = 13.sp
                            )
                        }
                    } else {
                        LazyColumn(modifier = Modifier.weight(1f)) {
                            items(discoveredDevices) { device ->
                                Card(
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .padding(vertical = 4.dp)
                                        .clickable(enabled = selectedFiles.isNotEmpty() && !isSending) {
                                            scope.launch {
                                                isSending = true
                                                statusMessage = "AirDropping ${selectedFiles.size} file(s) to ${device.instanceName}..."
                                                val paths = selectedFiles.map { it.absolutePath }
                                                val res = AirDropBridge.sendFilesToDevice(
                                                    targetUrl = device.targetUrl,
                                                    files = paths,
                                                    senderName = deviceName,
                                                    senderModel = "Pixel"
                                                )
                                                isSending = false
                                                if (res.success) {
                                                    statusMessage = "✅ Successfully AirDropped to ${device.instanceName}!"
                                                    Toast.makeText(context, "AirDrop Successful!", Toast.LENGTH_SHORT).show()
                                                } else {
                                                    statusMessage = "❌ AirDrop failed: ${res.error}"
                                                    Toast.makeText(context, "Failed: ${res.error}", Toast.LENGTH_LONG).show()
                                                }
                                            }
                                        },
                                    colors = CardDefaults.cardColors(containerColor = Color(0xFF2C2C2E))
                                ) {
                                    Row(
                                        modifier = Modifier
                                            .fillMaxWidth()
                                            .padding(12.dp),
                                        verticalAlignment = Alignment.CenterVertically
                                    ) {
                                        Icon(
                                            imageVector = Icons.Default.PhoneIphone,
                                            contentDescription = "Apple Device",
                                            tint = Color(0xFF007AFF),
                                            modifier = Modifier.size(32.dp)
                                        )
                                        Spacer(modifier = Modifier.width(12.dp))
                                        Column(modifier = Modifier.weight(1f)) {
                                            Text(device.instanceName, fontWeight = FontWeight.Bold, fontSize = 15.sp)
                                            Text(device.targetUrl, color = Color.Gray, fontSize = 12.sp)
                                        }
                                        if (selectedFiles.isNotEmpty()) {
                                            Text("Tap to Send", color = Color(0xFF007AFF), fontSize = 13.sp, fontWeight = FontWeight.SemiBold)
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Spacer(modifier = Modifier.height(12.dp))

            // Status bar
            Card(
                shape = RoundedCornerShape(8.dp),
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                modifier = Modifier.fillMaxWidth()
            ) {
                Text(
                    text = statusMessage,
                    modifier = Modifier.padding(12.dp),
                    fontSize = 13.sp,
                    color = MaterialTheme.colorScheme.onSurface
                )
            }
        }
    }
}

fun copyUriToCache(context: android.content.Context, uri: Uri): File? {
    return try {
        val returnCursor = context.contentResolver.query(uri, null, null, null, null)
        val nameIndex = returnCursor?.getColumnIndex(android.provider.OpenableColumns.DISPLAY_NAME) ?: -1
        returnCursor?.moveToFirst()
        val name = if (nameIndex != -1) returnCursor?.getString(nameIndex) ?: "file_${System.currentTimeMillis()}" else "file_${System.currentTimeMillis()}"
        returnCursor?.close()

        val tempFile = File(context.cacheDir, name)
        context.contentResolver.openInputStream(uri)?.use { input ->
            FileOutputStream(tempFile).use { output ->
                input.copyTo(output)
            }
        }
        tempFile
    } catch (e: Exception) {
        e.printStackTrace()
        null
    }
}
