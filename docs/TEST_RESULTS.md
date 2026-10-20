# Universal AirDrop Test Results Log

This document records exact test commands, device parameters, outputs, and validation evidence.

---

## Run 1: Unit Test Suite
- **Date**: 2026-10-20
- **Command**: `cargo test --all`
- **Output**:
  ```
  running 7 tests
  test ble::tests::test_ble_everyone_encode_parse ... ok
  test contacts::tests::test_phone_normalization ... ok
  test contacts::tests::test_email_normalization ... ok
  test ble::tests::test_ble_nearby_action_encode_parse ... ok
  test contacts::tests::test_hash_match ... ok
  test cpio::tests::test_cpio_roundtrip ... ok
  test tls::tests::test_cert_generation ... ok
  test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
  ```
- **Result**: **PASS**

---

## Run 2: Multi-File End-to-End Transfer Verification
- **Date**: 2026-10-20
- **Receiver**: `airdrop-cli receive --port 8776 --output ./test_downloads`
- **Sender**: `airdrop-cli send --target 127.0.0.1:8776 test_transfer/note.txt test_transfer/binary_data.bin`
- **Verification Command**:
  ```bash
  cmp test_transfer/binary_data.bin test_downloads/binary_data.bin && cmp test_transfer/note.txt test_downloads/note.txt
  ```
- **Output**:
  ```
  FILES ARE 100% IDENTICAL!
  ```
- **Transferred Files**:
  - `note.txt`: 13 bytes
  - `binary_data.bin`: 51,200 bytes
- **Result**: **PASS** (Zero bit errors, verified bit-for-bit via `cmp`)

---

## Run 3: Hardware Inventory via ADB
- **Date**: 2026-10-20
- **Command**: `adb devices -l`
- **Output**:
  ```
  0020965A9001588 device usb:1048576X product:GalagaIND model:A001 device:Galaga transport_id:1
  ```
- **Properties Query**:
  - Android Version: 16 (SDK 36)
  - Model: A001 (Nothing Phone)
  - ABI: `arm64-v8a`
  - SELinux: `Enforcing`
  - IP Address on wlan0: `192.168.29.229`
- **Ping from Mac (`192.168.29.18`)**:
  - RTT: 6.008 ms, 0.0% packet loss.
- **Result**: **CONFIRMED**

---

## Run 4: Real Cross-Device Transfer (Mac → Nothing Phone A001)
- **Date**: 2026-10-20
- **Sender**: MacBook Air M1 (`192.168.29.18`) via `airdrop-cli send`
- **Receiver**: Nothing Phone A001 (`192.168.29.229:8770`, Android 16) via on-device AirDrop app daemon
- **Files Transferred**:
  - `mac_binary_payload.bin` (102,400 bytes, random binary data)
  - `test_to_android.txt` (65 bytes)
- **Send Command**:
  ```bash
  ./target/debug/airdrop-cli send --target 192.168.29.229:8770 \
    ./test_to_android.txt ./mac_binary_payload.bin \
    --name "MacBook Air" --model "MacBookAir"
  ```
- **Integrity Verification**:
  ```bash
  sha256sum ./mac_binary_payload.bin && adb shell sha256sum /sdcard/Download/AirDrop/mac_binary_payload.bin
  ```
- **SHA-256 Hashes**:
  - Mac: `4961ada1dea2deef2ec54845a04618b2954e96ff662a639d97cd07995ab83320`
  - Android: `4961ada1dea2deef2ec54845a04618b2954e96ff662a639d97cd07995ab83320`
- **Result**: **PASS** (100% byte-for-byte match)

---

## Run 5: Real Cross-Device Transfer (Nothing Phone A001 → Mac)
- **Date**: 2026-10-20
- **Sender**: Nothing Phone A001 (`192.168.29.229`, Android 16) via Compose UI AirDrop App
- **Receiver**: MacBook Air M1 (`192.168.29.18:8780`) via `airdrop-cli receive`
- **File Transferred**: `screen.png` (152,172 bytes, device screenshot)
- **Android UI Action**:
  - Selected `screen.png` from file picker.
  - Targeted Mac AirDrop receiver daemon at `192.168.29.18:8780`.
  - Tapped `Send`.
  - UI displayed: `✅ Successfully AirDropped to https://192.168.29.18:8780!`
- **Integrity Verification**:
  ```bash
  sha256sum /Users/hritish/Documents/REA/mac_received/screen.png && adb shell sha256sum /sdcard/screen.png
  ```
- **SHA-256 Hashes**:
  - Mac: `576ab90d9f9ff3ea5d5e5f4619208986fdf73924ee9e140cf7e26b947b6d5791`
  - Android: `576ab90d9f9ff3ea5d5e5f4619208986fdf73924ee9e140cf7e26b947b6d5791`
- **Result**: **PASS** (100% byte-for-byte match)

---

## Run 6: Automated Security & Regression Suite
- **Date**: 2026-10-20
- **Command**: `./scripts/verify_all.sh`
- **Checks Executed**:
  - Rust format (`cargo fmt --check`)
  - Workspace test suite (`cargo test --all`)
  - Path traversal security test (`cpio::tests::test_path_traversal_prevention`)
  - BLE contact hash list validity test (`ble::tests::test_ble_empty_contacts_not_everyone`)
  - Multi-file CPIO pack/unpack roundtrip
  - CLI multi-file local loopback transfer
- **Result**: **PASS** (All tests green)

---

## Run 7: 24/7 Background Screen-Off Transfer & MediaStore Auto-Indexing
- **Date**: 2026-10-20
- **Android Target**: Nothing Phone A001 (`192.168.29.229:8770`, Android 16, SDK 36) running `AirDropForegroundService`.
- **Target State**: Screen locked in DOZE state (`mScreenState=DOZE`, screen powered off).
- **Service Configuration**:
  - Service Type: `FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE`
  - Locks Held: `WifiManager.MulticastLock` + `PowerManager.WakeLock` (PARTIAL_WAKE_LOCK)
  - Advertised Apple Persona: `MacBookPro18,1` (Apple M1 Pro 16-inch MacBook Pro)
  - Advertised mDNS Instance: `44197d98940c._airdrop._tcp.local.`
  - BLE Advertisements: Dual active (AirDrop Type `0x05` + NearbyAction Type `0x10`)
- **mDNS Resolution on macOS 15**:
  ```text
  Lookup 44197d98940c._airdrop._tcp.local.
  44197d98940c._airdrop._tcp.local. can be reached at Android_WTQIFE5R.local.:8770 (interface 11)
   dv=1.0 flags=1 model=MacBookPro18,1 phash=0000000000000000000000000000000000000000000000000000000000000000
  ```
- **Transfer Command Executed from Mac**:
  ```bash
  ./target/debug/airdrop-cli send --target "192.168.29.229:8770" target/test_background.txt --name "MacBookPro18,1"
  ```
- **Integrity Verification**:
  ```bash
  shasum -a 256 target/test_background.txt
  # 4bed871c5524929f6498e178aa91dcd008185f0559e7921d312385b3f9c8ba1d  target/test_background.txt

  adb -s 0020965A9001588 shell sha256sum /storage/emulated/0/Android/data/com.airdrop.android/files/test_background.txt
  # 4bed871c5524929f6498e178aa91dcd008185f0559e7921d312385b3f9c8ba1d  .../test_background.txt
  ```
- **On-Device MediaStore Auto-Indexing Logcat Trace**:
  ```text
  AirDropService: New file completed: test_background.txt (72 bytes)
  MediaStoreUtils: Indexing received file test_background.txt (text/plain)
  MediaStoreUtils: Successfully copied to MediaStore: content://media/external/downloads/1000060316
  ```
- **Result**: **PASS** (100% byte-for-byte SHA-256 match received while phone was locked in background with automatic MediaStore indexing).

