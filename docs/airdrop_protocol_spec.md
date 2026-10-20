# Apple AirDrop Protocol Specification & Reverse-Engineering Architecture
**Derived from REA Analysis of macOS `sharingd` & Apple Continuity Stack**

---

## 1. Architectural Overview

AirDrop is Apple's proprietary peer-to-peer file sharing protocol designed across multiple network abstraction layers. It combines **Bluetooth Low Energy (BLE)** for initial proximity discovery, **Multicast DNS (mDNS / Bonjour)** for IP-level service discovery, **TLS 1.2/1.3** for encrypted transport, and a **REST-like HTTP server** for session negotiation and file streaming.

```
+-------------------------------------------------------------------------+
|                           Application Layer                             |
|               CPIO / BOM Archive + File Stream Transfer                 |
+-------------------------------------------------------------------------+
|                           HTTP Session Layer                            |
|             POST /Discover   |   POST /Ask   |   POST /Upload           |
+-------------------------------------------------------------------------+
|                           Security / Crypto                             |
|           TLS 1.2/1.3 (Self-Signed or Apple ID IDS Verified)            |
+-------------------------------------------------------------------------+
|                          Service Discovery                              |
|                mDNS / Bonjour: _airdrop._tcp. on Port 8770+             |
+-------------------------------------------------------------------------+
|                         Proximity Discovery                             |
|            BLE Advertising: Apple Inc. (0x004c), Types 0x05 / 0x10      |
+-------------------------------------------------------------------------+
|                     Physical & Data Link Transport                      |
|      Option A: Standard Wi-Fi LAN / Hotspot (IPv4 / IPv6 Link-Local)    |
|      Option B: Apple Wireless Direct Link (AWDL - 802.11 Action Frames) |
+-------------------------------------------------------------------------+
```

---

## 2. Proximity Discovery Layer (BLE Advertisements)

AirDrop uses Apple Bluetooth Low Energy (BLE) advertisements to detect nearby devices before establishing high-speed Wi-Fi connections.

### Company Identifier
* **Company ID**: `0x004C` (Apple, Inc.) in the Manufacturer Specific Data payload.

### Sub-Type 0x05 (AirDrop Discovery)
When a device is discoverable or opening the ShareSheet:
* **Byte 0**: `0x05` (Continuity Type: AirDrop)
* **Byte 1**: Length of payload (typically 18 bytes)
* **Bytes 2–9**: Zero padding / state flags
* **Bytes 10–17**: Contact Hashes:
  * In **"Everyone"** mode: These bytes are zeros.
  * In **"Contacts Only"** mode: 2-byte or 3-byte truncated SHA-256 hashes of the user's phone numbers (in E.164 format, e.g., `+14155552671`) and email addresses (normalized lowercase).

### Sub-Type 0x10 (Nearby Action)
Used in newer iOS/macOS releases (Continuity / NameDrop / Proximity Sharing):
* **Byte 0**: `0x10` (Nearby Action)
* **Byte 1**: Length
* **Byte 2**: Action Flags
* **Byte 3**: Action Type (`0x01` = AirDrop / File Sharing)
* **Bytes 4+**: Authentication tag and ephemeral session token.

---

## 3. Service Discovery Layer (mDNS / Bonjour)

Once BLE indicates a nearby peer, or whenever devices share an active Wi-Fi network (or AWDL interface), they announce their receiver daemon via mDNS.

### Service Identity
* **Service Type**: `_airdrop._tcp.local.`
* **Service Instance Name**: A 12-character hexadecimal string derived from the receiver's MAC address or ephemeral node UUID (e.g., `ac84747cd14b._airdrop._tcp.local.`).
* **Port**: Dynamically allocated or static ephemeral port (typically between `8770` and `8779`).

### TXT Record Attributes
The mDNS TXT record carries state metadata:
* `flags`: Decimal integer bitfield:
  * `0x01`: Discoverable / active receiver.
  * `0x02`: Contacts Only mode enabled.
  * `0x04`: Supports cellular data / direct IP fallback (`EnableDirectIP`).
* `phash`: Personal contact hash (Salted SHA-256 of active Apple ID identity).
* `airdrop`: Capability / protocol version string (e.g. `1.0`).

---

## 4. Cryptographic & Transport Layer (TLS)

All communication between peers takes place over HTTPS on the advertised mDNS port.

### TLS Configuration
* **Version**: TLS 1.2 or TLS 1.3.
* **Cipher Suites**: Modern AEAD suites (`TLS_AES_128_GCM_SHA256`, `TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256`, `ECDHE-ECDSA-AES128-GCM-SHA256`).
* **Certificates**:
  * In **"Everyone"** mode: Self-signed X.509 certificate generated on the fly. The Apple client does not enforce CA trust in Everyone mode; it verifies public-key consistency throughout the session.
  * In **"Contacts Only"** mode: The certificate contains Apple ID validation extensions signed by Apple ID Identity Services (IDS).

### Contact Hash Generation Rule
Contact hashes match peers in the address book:
1. **Email Normalization**: Lowercase trimmed ASCII string (e.g., `john.doe@example.com`).
2. **Phone Normalization**: E.164 international format without spaces or symbols (e.g., `+14155552671`).
3. **Digest**: `SHA256(normalized_string)`.
4. **Truncation**: For BLE advertisements, the first 2 or 3 bytes are extracted; for TLS/HTTP verification, the full 32-byte hash is compared.

---

## 5. HTTP Protocol & Message Schemas

Communication consists of three core HTTP transactions. Payload formats accept both JSON and Apple Binary Plist (BPlist).

### 1. `POST /Discover`
Handshake and device capability negotiation.

**Request Body (JSON):**
```json
{
  "senderRecordData": "<optional base64 encoded sender certificate>",
  "receiverRecordData": "<optional base64 encoded target certificate>"
}
```

**Response Body (JSON):**
```json
{
  "status": "success",
  "deviceSupportFlags": 1,
  "protocolVersion": "1.0"
}
```

---

### 2. `POST /Ask`
Sender asks receiver for permission to transfer one or more files. This triggers the AirDrop notification on the recipient's screen.

**Request Body (JSON):**
```json
{
  "SenderComputerName": "Android Device",
  "SenderModelName": "Pixel",
  "SenderID": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "BundleID": "com.apple.mobileslideshow",
  "ConvertMediaFormats": false,
  "Files": [
    {
      "FileName": "photo.jpg",
      "FileType": "public.jpeg",
      "FileBomPath": "./photo.jpg",
      "FileIsDirectory": false,
      "FileSize": 2048576
    }
  ],
  "itemsDescription": "1 Photo"
}
```

**Recipient Action:**
* The Apple device displays: *"AirDrop: Android Device would like to share 1 Photo. [Decline] [Accept]"*.
* If **Accepted**, receiver returns HTTP `200 OK`:
```json
{
  "status": "Accepted",
  "nextAttemptDelayMS": 0
}
```
* If **Declined**, receiver returns HTTP `200 OK` (or `403`):
```json
{
  "status": "Declined"
}
```

---

### 3. `POST /Upload`
Upon receiving `"Accepted"` from `/Ask`, the sender streams the files to `/Upload`.

* **Request Headers**:
  * `Content-Type: application/octet-stream`
  * `X-AirDrop-Format: cpio`
* **Payload**:
  * Streamed standard POSIX CPIO archive (odc or newc format) containing file metadata and binary content.
* **Response**:
  * HTTP `200 OK` once all bytes are verified.

---

## 6. Android Implementation Architecture

```
+---------------------------------------------------------------------+
|                     Android App (Jetpack Compose UI)                |
|           Send Screen  |  Receive Screen  |  Nearby Device List     |
+---------------------------------------------------------------------+
|                    JNI Bridge (Rust FFI Interface)                  |
|    discover_peers() | send_file() | start_server() | stop_server()  |
+---------------------------------------------------------------------+
|                     Rust Core Engine (`airdrop-core`)               |
|  - BLE Engine: Apple 0x004C Packet Encoder / Decoder                |
|  - mDNS Engine: _airdrop._tcp Client & Server (trust-dns / zeroconf)|
|  - TLS & HTTP: Axum / Hyper / Rustls HTTPS Server & Client          |
|  - Archive Engine: Streaming CPIO Archive Builder & Extractor       |
|  - Contact Matching: E.164 & Email SHA-256 Hasher                  |
+---------------------------------------------------------------------+
|                     Network Transport Layer                         |
|  Layer 1: Standard Wi-Fi Socket (Stock unrooted Android / Hotspot)  |
|  Layer 2: AWDL / TUN Raw Frame Interface (Rooted / Custom Wi-Fi)    |
+---------------------------------------------------------------------+
```

---

## 7. Operational Modes on Android

1. **Unrooted Standard Wi-Fi LAN**:
   * Both devices are connected to the same Wi-Fi router, or Android runs a standard Portable Wi-Fi Hotspot that the iPhone/Mac joins.
   * Android uses standard Android `NsdManager` / `multicast-dns` and standard TCP sockets.
   * Works on 100% of Android devices without root or custom firmware.

2. **Rooted Peer-to-Peer AWDL (Advanced)**:
   * Requires a rooted phone with Wi-Fi monitor/injection or OWL (Open Wireless Link) kernel module.
   * Android broadcasts 802.11 action frames on social channels 6 / 44 to negotiate out-of-band peer-to-peer data links with no shared Wi-Fi network.
