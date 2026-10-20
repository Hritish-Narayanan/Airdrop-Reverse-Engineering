# Universal AirDrop Architecture & Data Flow

This document outlines the system architecture, internal components, communication interfaces, and cross-platform data flow for Universal AirDrop (Android ↔ Apple).

---

## 1. High-Level Component Diagram

```
+---------------------------------------------------------------------------------+
|                                 Android Client                                  |
|                                                                                 |
|  +-------------------------+     +---------------------+     +---------------+  |
|  |       MainActivity      |     |  AirDropBleManager  |     | AirDropNsdMgr |  |
|  |  (Jetpack Compose UI)   |     |   (BLE Advertiser)  |     | (mDNS Bonjour)|  |
|  +------------+------------+     +----------+----------+     +-------+-------+  |
|               |                             |                        |          |
|               v                             v                        v          |
|  +---------------------------------------------------------------------------+  |
|  |                           AirDropBridge (Kotlin)                          |  |
|  +--------------------------------------+------------------------------------+  |
|                                         | JNI (C-ABI)                           |
+-----------------------------------------|---------------------------------------+
                                          v
+---------------------------------------------------------------------------------+
|                       airdrop-jni (Rust cdylib layer)                           |
|       Java_com_airdrop_android_AirDropBridge_{init, sendFiles, startReceiver}   |
+-----------------------------------------+---------------------------------------+
                                          |
                                          v
+---------------------------------------------------------------------------------+
|                              airdrop-core (Rust)                                |
|                                                                                 |
|   +-------------------+    +-------------------+    +----------------------+    |
|   |    ble.rs         |    |   contacts.rs     |    |      tls.rs          |    |
|   | (Apple 0x004c)    |    | (SHA-256 Hashes)  |    | (P-256 ECDSA X.509)  |    |
|   +-------------------+    +-------------------+    +----------------------+    |
|   |    mdns.rs        |    |   client.rs       |    |      server.rs       |    |
|   | (_airdrop._tcp.)  |    | (/Ask, /Upload)   |    | (HTTPS Daemon)       |    |
|   +-------------------+    +-------------------+    +----------------------+    |
|   |    cpio.rs        |    |   protocol.rs     |                            |    |
|   | (POSIX Archive)   |    | (JSON / Schemas)  |                            |    |
|   +-------------------+    +-------------------+                            |    |
+---------------------------------------------------------------------------------+
                                          |
                                          | Wi-Fi LAN / Hotspot / AWDL
                                          v
+---------------------------------------------------------------------------------+
|                              Apple Device (Target)                              |
|                                                                                 |
|   +-------------------------------------------------------------------------+   |
|   |                    /usr/libexec/sharingd (macOS / iOS)                  |   |
|   |                                                                         |   |
|   |   - BLE Discovery (Apple Type 0x05 / 0x10)                              |   |
|   |   - Bonjour Service Registration (_airdrop._tcp.)                       |   |
|   |   - Ephemeral TLS Session (Port 8770)                                   |   |
|   |   - REST API Endpoints (/Discover, /Ask, /Upload)                       |   |
|   |   - Native AirDrop Prompt & UI                                          |   |
|   +-------------------------------------------------------------------------+   |
+---------------------------------------------------------------------------------+
```

---

## 2. Sequence Diagram: Android Sending to Apple Device

```mermaid
sequenceDiagram
    autonumber
    actor User as Android User
    participant App as Android UI (MainActivity)
    participant Core as airdrop-core (Rust)
    participant Apple as Apple Device (sharingd)

    User->>App: Select files & tap Scan
    App->>Core: discover_airdrop_peers(timeout: 5s)
    Core->>Apple: mDNS query for _airdrop._tcp.local.
    Apple-->>Core: mDNS response (Instance ID, Port, IP)
    Core-->>App: DiscoveredPeer list
    App-->>User: Display Apple Device in list

    User->>App: Tap Apple Device to Send
    App->>Core: send_files(target_url, file_paths)
    Core->>Apple: TLS 1.3 Handshake (Ephemeral P-256)
    Core->>Apple: POST /Discover
    Apple-->>Core: 200 OK (Capabilities)

    Core->>Apple: POST /Ask (Files manifest, Sender name & model)
    Note over Apple: Apple device sounds chime & displays native AirDrop prompt
    Apple-->>Core: 200 OK (User accepted prompt)

    Core->>Core: Pack files into POSIX CPIO archive (streaming)
    Core->>Apple: POST /Upload (CPIO stream)
    Apple-->>Core: 200 OK (Files unpacked)
    Core-->>App: Transfer successful
    App-->>User: Visual success confirmation
```

---

## 3. Sequence Diagram: Apple Device Sending to Android

```mermaid
sequenceDiagram
    autonumber
    actor AppleUser as Apple User
    participant Apple as Apple Device (sharingd)
    participant BLE as Android BLE Advertiser
    participant Server as Android HTTPS Server (airdrop-core)
    participant App as Android UI (MainActivity)
    actor AndroidUser as Android User

    AndroidUser->>App: Toggle AirDrop Receiver ON
    App->>Server: run_airdrop_server(port: 8770)
    App->>BLE: Start advertising Apple 0x004c (Type 0x05)
    App->>Server: Register mDNS _airdrop._tcp.local.

    AppleUser->>Apple: Opens AirDrop Share Sheet
    Apple->>Apple: Detects Android BLE advertisement
    Apple->>Server: Discovers mDNS _airdrop._tcp.local.
    AppleUser->>Apple: Taps Android Device icon

    Apple->>Server: TLS Handshake
    Apple->>Server: POST /Discover
    Server-->>Apple: 200 OK (Receiver Computer Name)

    Apple->>Server: POST /Ask (File manifest)
    Server->>App: Notify incoming transfer prompt
    AndroidUser->>App: Tap Accept
    Server-->>Apple: 200 OK

    Apple->>Server: POST /Upload (POSIX CPIO stream)
    Server->>Server: Stream unpack directly to Downloads/AirDrop/
    Server-->>Apple: 200 OK
    Server-->>App: Files saved & notify user
```
