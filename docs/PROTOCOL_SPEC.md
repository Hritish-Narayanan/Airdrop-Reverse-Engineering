# Universal AirDrop Protocol Specification

Derived from static & runtime analysis of macOS `sharingd` (Mach-O arm64e), OpenDrop, OWL, and network observations.

---

## 1. Protocol Confidence & Uncertainty Matrix

| Layer / Feature | Implementation State | Evidence Level | Notes |
|---|---|---|---|
| **BLE Type 0x05 (Everyone)** | Implemented | **CONFIRMED** | Extracted from `sharingd`, validated in OpenDrop/OWL |
| **BLE Type 0x05 (Contacts)** | Implemented | **CONFIRMED** | 8-byte contact hash area with E.164 + email SHA-256 prefixes |
| **BLE Type 0x10 (NearbyAction)**| Implemented | **STRONG HYPOTHESIS** | Proximity action type 0x01 |
| **mDNS Service Type** | Implemented | **CONFIRMED** | `_airdrop._tcp.local.` on port 8770+ |
| **mDNS TXT Records** | Implemented | **CONFIRMED** | `flags=0x01` |
| **TLS Certificate Format** | Implemented | **CONFIRMED** | Self-signed X.509 P-256 ECDSA |
| **REST: POST /Discover** | Implemented | **CONFIRMED** | Returns receiver metadata |
| **REST: POST /Ask** | Implemented | **CONFIRMED** | Manifest dictionary: `SenderComputerName`, `Files` |
| **REST: POST /Upload** | Implemented | **CONFIRMED** | Raw streaming archive |
| **Container: POSIX CPIO** | Implemented | **CONFIRMED** | Standard `odc`/`newc` archive ending with `TRAILER!!!` |
| **Container: DVZip / BOM** | In Progress | **UNDER INVESTIGATION** | Newer iOS 17+ adaptive compression framing |
| **Binary Property Lists (bplist)**| Implemented | **CONFIRMED & UNIT TESTED** | Automatic `bplist00`, XML plist, and JSON payload deserialization |

---

## 2. BLE Proximity Discovery

### Manufacturer Data Format
- **Company ID**: `0x004C` (Apple, Inc., little-endian `4C 00` in BLE advertising PDU).
- **Sub-Type 0x05 (AirDrop)**:
  - Byte 0: `0x05` (Type)
  - Byte 1: `0x12` (Length: 18 bytes)
  - Bytes 2–9: Zero padding / state flags (`0x00`...)
  - Bytes 10–17: Contact Hashes:
    - *Everyone Mode*: All 8 bytes are `0x00`.
    - *Contacts Only Mode*: Four 2-byte truncated SHA-256 hashes of normalized contact identities.
- **Contact Normalization**:
  - Phone: Strip non-digits, format with leading `+` and country code (E.164, e.g. `+14155552671`).
  - Email: Lowercase, trim whitespace (e.g. `user@example.com`).
  - Hash: `SHA256(contact)[0..2]`.

---

## 3. Network Transport & Bonjour Service

- **Service Registration**:
  - Service Type: `_airdrop._tcp.local.`
  - Domain: `local.`
  - Instance Name: 12-hex character ID (e.g. MAC address lowercase `0123456789ab`).
  - Port: Dynamically bound (conventionally `8770`–`8779`).
  - TXT Record Keys:
    - `flags`: `0x01` (standard), `0x81` (extended capabilities).

---

## 4. HTTPS Control & Data Exchange

All endpoints are served over TLS 1.2 / TLS 1.3 with ECDSA P-256 self-signed certificates.

### 4.1 POST /Discover
- **Purpose**: Query recipient capabilities and identify sender.
- **Request Headers**:
  - `Content-Type: application/json` or `application/x-apple-binary-plist`
- **Request Body**:
  ```json
  {
    "Flags": 1
  }
  ```
- **Response Body**:
  ```json
  {
    "ReceiverComputerName": "Target Apple Device",
    "ReceiverMediaCapabilities": {}
  }
  ```

### 4.2 POST /Ask
- **Purpose**: Prompt recipient to accept incoming file transfer.
- **Request Body**:
  ```json
  {
    "SenderComputerName": "Pixel 8 Pro",
    "SenderModelName": "Pixel",
    "SenderID": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
    "BundleID": "com.apple.AirDrop",
    "ConvertMediaFormats": false,
    "Files": [
      {
        "FileName": "photo.jpg",
        "FileType": "public.jpeg",
        "FileBomPath": "photo.jpg",
        "FileIsDirectory": false,
        "FileSize": 1048576
      }
    ]
  }
  ```
- **Response**:
  - `200 OK`: Transfer accepted by recipient.
  - `401 Unauthorized` / `403 Forbidden`: Transfer declined or timed out.

### 4.3 POST /Upload
- **Purpose**: Transmit file archive.
- **Request Headers**:
  - `Content-Type: application/octet-stream`
  - `Transfer-Encoding: chunked` (optional/recommended for streaming large files)
  - `Expect: 100-continue`
- **Payload**: POSIX CPIO archive (`odc` octet-oriented format, magic `070707`), ending with filename `TRAILER!!!`.
- **Response**:
  - `200 OK`: Archive received and extracted successfully.
