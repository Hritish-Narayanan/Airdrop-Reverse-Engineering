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

