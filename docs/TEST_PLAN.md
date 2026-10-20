# Universal AirDrop Test Plan & Strategy

This document defines the multi-tier testing strategy, acceptance criteria, test environments, and regression verification rules for the project.

---

## 1. Test Layers

```
+--------------------------------------------------------------------------+
| Tier 6: Physical Interoperability Tests (Android ↔ Mac ↔ iPhone / iPad)  |
+--------------------------------------------------------------------------+
| Tier 5: Device Smoke Tests (ADB install, permissions, BLE advertise)     |
+--------------------------------------------------------------------------+
| Tier 4: Local Integration Tests (Receiver daemon ↔ Sender client)        |
+--------------------------------------------------------------------------+
| Tier 3: Security & Fuzz Tests (Path traversal, oversized archives)       |
+--------------------------------------------------------------------------+
| Tier 2: Golden Fixture Tests (CPIO packing/unpacking, BLE manufacturer)  |
+--------------------------------------------------------------------------+
| Tier 1: Unit Tests (Contacts hashing, TLS cert generation, mDNS records) |
+--------------------------------------------------------------------------+
```

---

## 2. Test Execution Matrix

### Tier 1: Unit Tests
- `contacts::tests::test_phone_normalization`: E.164 conversion (+ country code).
- `contacts::tests::test_email_normalization`: Lowercasing, whitespace trimming.
- `contacts::tests::test_hash_match`: Truncated SHA-256 hash match against reference vector.
- `ble::tests::test_ble_everyone_encode_parse`: BLE type 0x05 everyone mode packet.
- `ble::tests::test_ble_nearby_action_encode_parse`: BLE type 0x10 NearbyAction packet.
- `tls::tests::test_cert_generation`: Ephemeral self-signed P-256 certificate generation.
- `cpio::tests::test_cpio_roundtrip`: Pack and unpack memory buffers bit-for-bit.

### Tier 2: Golden Fixture Tests
- Test CPIO archive compatibility with standard Unix `cpio -idmv` and Apple `cpio`.
- Verify trailing marker `TRAILER!!!` compliance.

### Tier 3: Security & Boundary Tests
- Path Traversal: Files with names like `../../etc/passwd` or `/root/evil` MUST be stripped or rejected.
- Null byte injection: Filenames containing `\0` MUST be rejected.
- Memory Boundedness: Stream 100MB+ without RSS memory ballooning.

### Tier 4: Local Integration Tests
- Launch `airdrop-cli receive` on port 8775.
- Transmit multi-file payload using `airdrop-cli send`.
- Verify with `cmp` that output files match source byte-for-byte.

### Tier 5: Android Device Tests (via ADB)
- Verify `libairdrop_jni.so` loads on Android 16 (`arm64-v8a`) without `UnsatisfiedLinkError`.
- Verify BLE advertisement starts via `BluetoothLeAdvertiser` without crashing.
- Verify Wi-Fi multicast lock acquisition and mDNS registration via `NsdManager`.
- Verify file selection and storage output in `Download/AirDrop/`.

### Tier 6: Physical Apple Interoperability
- Test discovery from macOS Finder / AirDrop window.
- Test discovery from iOS AirDrop Share Sheet.
- Execute real file transfer and verify received file on both ends.
