# Universal AirDrop: Actionable Engineering Checklist

Status Legend:
- `NOT STARTED`: Work has not commenced.
- `IN PROGRESS`: Active development or testing underway.
- `BLOCKED`: Blocked on documented technical or hardware constraint.
- `VERIFIED`: Proven complete with reproducible test evidence.
- `PARTIALLY VERIFIED`: Subsets pass, edge cases remaining.

---

## Task List

### Phase 0: Baseline & Environment Audit
- [x] **[VERIFIED]** Identify development workstation specs (M1 MacBook Air, macOS 15, Rust 1.98.1).
- [x] **[VERIFIED]** Identify connected Android device (`0020965A9001588`, Nothing Phone A001, Android 16, SDK 36, `arm64-v8a`, Wi-Fi `192.168.29.229`).
- [x] **[VERIFIED]** Verify workspace layout, Git repository state, and baseline test suite pass.
- [x] **[VERIFIED]** Install Android NDK 26.1 and `cargo-ndk` for native Android cross-compilation.

### Phase 1: Protocol Reverse Engineering
- [x] **[VERIFIED]** Reverse-engineer `/usr/libexec/sharingd` endpoints (`/Discover`, `/Ask`, `/Upload`).
- [x] **[VERIFIED]** Implement POSIX CPIO archive pack & unpack with `TRAILER!!!` termination.
- [x] **[VERIFIED]** Implement SHA-256 phone/email contact normalization and truncated hashing.
- [x] **[VERIFIED]** Verify protocol serialization and state machine against reference captures.

### Phase 2: Native P2P Transport Feasibility (AWDL)
- [x] **[VERIFIED]** Audit OWL / OpenDrop AWDL requirements against stock Android 16 kernel/SELinux.
- [x] **[VERIFIED]** Document AWDL vs Wi-Fi Direct vs Infrastructure LAN trade-offs in `docs/DECISIONS.md` (ADR-002).
- [x] **[VERIFIED]** Formulate hybrid transport architecture (Standard Wi-Fi LAN / Hotspot for zero-root consumer Android).

### Phase 3: BLE & Bonjour Discovery
- [x] **[VERIFIED]** BLE advertisement generator (Company ID `0x004c`, Type `0x05`, Type `0x10`).
- [x] **[VERIFIED]** Fix BLE contact hash list: resolved 2-byte chunk agreement and prevented empty list from advertising Everyone mode.
- [x] **[VERIFIED]** Align mDNS TXT record properties between Kotlin `AirDropNsdManager` and Rust `airdrop-core`.
- [x] **[VERIFIED]** Test Android BLE advertiser broadcasting live on Nothing Phone (A001) via Android BLE API.

### Phase 4: Identity, Trust & Recipient Consent
- [x] **[VERIFIED]** Implement explicit recipient consent toggle in UI (defaulting to user approval required).
- [x] **[VERIFIED]** Fix `/Ask` parsing fallback: unparsable requests now decline rather than auto-accept.
- [x] **[VERIFIED]** Ephemeral TLS (ECDSA P-256) session validation and peer identity parsing.

### Phase 5: Transfer Engine Hardening
- [x] **[VERIFIED]** POSIX CPIO pack and unpack roundtrip and transfer engine.
- [x] **[VERIFIED]** Implement path traversal containment (canonical containment checks rejecting `../` and null bytes).
- [x] **[VERIFIED]** SHA-256 checksum verification: 100% byte-for-byte match on cross-device transfers.

### Phase 6: Android Application Integration
- [x] **[VERIFIED]** Compile `libairdrop_jni.so` for `aarch64-linux-android` (arm64-v8a).
- [x] **[VERIFIED]** Verify JNI function signatures and exception safety.
- [x] **[VERIFIED]** Build debug APK via standalone Gradle 8.7 + OpenJDK 17 LTS.
- [x] **[VERIFIED]** Install APK on Nothing Phone (A001) using ADB (`0020965A9001588`).
- [x] **[VERIFIED]** Validate runtime permissions (Bluetooth, Wi-Fi multicast, storage) on Android 16.

### Phase 7: Real Apple Interoperability
- [x] **[VERIFIED]** Test Mac -> Android transfer over local network (`mac_binary_payload.bin`, SHA-256 verified).
- [x] **[VERIFIED]** Test Android -> Mac transfer over local network (`screen.png`, SHA-256 verified).
- [x] **[VERIFIED]** Record transfer throughput, logs, and SHA-256 verifications in `docs/TEST_RESULTS.md`.
- [ ] **[BLOCKED]** Physical iPhone / iPad interoperability testing (no physical iOS hardware connected; labeled NOT TESTED ON PHYSICAL APPLE HARDWARE).

### Phase 8: UX & Product Quality
- [x] **[VERIFIED]** Test Android Sharesheet intent reception (`ACTION_SEND`, `ACTION_SEND_MULTIPLE`).
- [x] **[VERIFIED]** Verify file selection, direct target input, and transfer status reporting in Compose UI.
- [x] **[VERIFIED]** Capture live screenshot of on-device success state on Nothing Phone (A001).

### Phase 9: Security Audit
- [x] **[VERIFIED]** Author `docs/SECURITY.md` threat model.
- [x] **[VERIFIED]** Create security unit tests against path traversal and oversized payloads.
- [x] **[VERIFIED]** Harden default transfer consent to explicit approval.

### Phase 10: Performance & Reliability
- [x] **[VERIFIED]** Benchmark transfer speeds for text, binary, and image payloads over Wi-Fi.
- [x] **[VERIFIED]** Bounded memory consumption during streaming CPIO extraction.

### Phase 11: Automated Continuous Validation
- [x] **[VERIFIED]** Create `scripts/verify_all.sh` running all unit, integration, and security tests.
- [x] **[VERIFIED]** 100% passing test suite across all workspace crates.

### Phase 12: Release Readiness
- [x] **[VERIFIED]** Package reproducible release build (`app-debug.apk` + cross-compiled JNI library).
- [x] **[VERIFIED]** Complete `docs/COMPATIBILITY_MATRIX.md` with final test evidence.
