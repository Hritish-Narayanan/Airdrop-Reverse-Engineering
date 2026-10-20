# Universal AirDrop: Persistent Engineering Invariants & Agent Instructions

This document governs all engineering activities, automated tasks, and architectural decisions for the **Universal AirDrop (Android ↔ Apple)** project.

---

## 1. Core Mission & Invariants

1. **Evidence Before Assumptions**:
   - Never speculate or fabricate protocol fields, test outputs, packet captures, or compatibility claims.
   - Any protocol assertion must be backed by reverse-engineering evidence (e.g. disassembly of macOS `sharingd`, packet captures, or verified reference implementations).
   - A feature is not verified merely because code compiles; it is verified only when executed against reproducible tests.

2. **Categorical Clarity**:
   - **Category A (Native AirDrop)**: Peer-to-peer interoperability interacting with Apple's native AirDrop daemon (`sharingd`) and AirDrop Share Sheet UI.
   - **Category B (Android Integration)**: Native Android integration via Sharesheet (`ACTION_SEND`, `ACTION_SEND_MULTIPLE`), Scoped Storage, and Background Services.
   - **Category C (Fallback Transfer)**: Direct LAN / Wi-Fi Hotspot transfer when AWDL or native peer-to-peer is unavailable.
   - **Category D (Quick Share)**: Google/Samsung Quick Share interoperability — completely separate protocol, never conflated with AirDrop.

3. **Safety & Non-Destructive Operations**:
   - Never run commands that factory reset, unlock bootloaders, root devices, or wipe user data.
   - Never commit private keys, personal certificates, user tokens, or personal identifiers.
   - Treat all remote peers, incoming files, and unauthenticated network data as untrusted.

4. **Resource Bounds & Hardware Considerations**:
   - The development workstation is an **Apple Silicon M1 MacBook Air**. Memory, disk I/O, and CPU must be conserved: prefer incremental builds, bounded parallelism, and bounded memory buffers.
   - Connected test device: **Nothing Phone (A001)**, Android 16 (SDK 36), `arm64-v8a`, SELinux Enforcing, USB Debugging authorized (`0020965A9001588`).

---

## 2. Protocol Boundaries & Standards

| Layer | Native AirDrop Protocol Standard | Fallback / Local LAN Implementation |
|---|---|---|
| **Discovery** | BLE (Company ID `0x004c`, Type `0x05`/`0x10`) + AWDL mDNS | BLE Advertiser + LAN mDNS (`_airdrop._tcp.local.`) |
| **Transport** | Apple Wireless Direct Link (AWDL) IPv6 link-local | Wi-Fi 802.11 Infrastructure LAN IPv4/IPv6 |
| **Security** | Ephemeral TLS (ECDSA P-256) + Contact Hash (SHA-256) | Ephemeral TLS (ECDSA P-256) + Custom Peer Verifier |
| **Control** | HTTP/1.1 REST (`POST /Discover`, `POST /Ask`) | HTTP/1.1 REST with strict JSON validation |
| **Data Framing** | POSIX CPIO (`odc`/`newc`) archive with trailing marker | POSIX CPIO streaming pack/unpack directly to disk |

---

## 3. Workflow & Verification Loop

For every milestone:
1. **Audit**: Inspect code and logs.
2. **Implement**: Small, coherent, bounded changes.
3. **Verify**: Run tests (`cargo test --all`, CLI transfer, ADB device smoke tests).
4. **Document**: Update `TODO.md`, `ROADMAP.md`, `docs/TEST_RESULTS.md`.
5. **Report**: Factual progress with reproduction commands and logs.
