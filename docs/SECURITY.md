# Universal AirDrop Security Model & Threat Assessment

---

## 1. Threat Model & Trust Boundaries

All remote devices discovered via BLE, mDNS, or network traffic are considered **untrusted peers**.
Incoming network traffic, headers, file manifests, and CPIO archive streams MUST be treated as potentially hostile input.

```
[ Untrusted Remote Network / BLE ]
               |
               v (TLS Handshake & REST Endpoints)
+-------------------------------------------------------------+
|                     Trust Boundary 1                        |
| - Validate Request Headers & Body Lengths                   |
| - Verify Ephemeral TLS Handshake / Certificate              |
+-------------------------------------------------------------+
               |
               v (/Ask Manifest Parsing)
+-------------------------------------------------------------+
|                     Trust Boundary 2                        |
| - Strict Sanitization of File Names                         |
| - Prompt User for Explicit Recipient Consent (No Auto-Accept)|
+-------------------------------------------------------------+
               |
               v (/Upload Archive Extraction)
+-------------------------------------------------------------+
|                     Trust Boundary 3                        |
| - Path Traversal Prevention (Zip/CPIO-Slip)                 |
| - Symlink Escape Prevention                                 |
| - Decompression / Disk Bomb Rate Limiting                   |
+-------------------------------------------------------------+
               |
               v (Local Storage)
[ Sandboxed Destination: Download/AirDrop/ ]
```

---

## 2. Specific Security Controls & Mitigations

### 2.1 Explicit Recipient Consent (No Silent Acceptance)
- **Vulnerability**: If incoming files are automatically accepted without user confirmation, malicious actors within Wi-Fi range could push unwanted or malicious files to the device.
- **Mitigation**: Every incoming `/Ask` request requires user approval in the UI. In the production app, `auto_accept` is strictly `false` by default, presenting a prominent confirmation dialog showing the sender's name and file manifest before `/Ask` returns 200 OK.

### 2.2 Path Traversal Prevention (CPIO-Slip)
- **Vulnerability**: A malicious CPIO archive could contain paths such as `../../../../system/bin/sh` or `/etc/shadow`.
- **Mitigation**:
  - The CPIO extraction engine strips all directory components and parent directory sequences (`../` or `..\\`).
  - Filenames are restricted to the base name: `Path::new(&entry.name).file_name()`.
  - Destination paths are checked via `canonicalize` to ensure the final target resides strictly within the configured download directory.
  - Absolute paths starting with `/` or drive letters are rejected.
  - Null bytes (`\0`) inside filenames trigger an immediate extraction abort.

### 2.3 Symlink & Hardlink Abuse
- **Vulnerability**: An archive could create a symlink to an arbitrary directory and extract subsequent files through the symlink.
- **Mitigation**:
  - The extraction engine strictly creates regular files (`std::fs::File::create`).
  - Symlink and hardlink creation is disabled in the archive extractor.

### 2.4 Resource Exhaustion & Memory Safety
- **Vulnerability**: Huge files (e.g. 50GB) buffered entirely into RAM could cause Out-Of-Memory (OOM) crashes.
- **Mitigation**:
  - Archive streaming: CPIO archives and uploads/downloads are processed in 64KB bounded chunks.
  - Memory consumption remains flat regardless of transferred file size.

### 2.5 Contact Identity Privacy
- **Vulnerability**: Full email addresses or phone numbers could be snooped if broadcast in plaintext over BLE.
- **Mitigation**:
  - AirDrop utilizes SHA-256 hashes truncated to 2 bytes (16 bits) or 3 bytes (24 bits) for contact discovery.
  - In "Everyone" mode, all contact hash fields are set to zeros, broadcasting zero personal identifiers.
