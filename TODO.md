# Universal AirDrop: Actionable Engineering Checklist

Status Legend:
- `NOT STARTED`: Work has not commenced.
- `IN PROGRESS`: Active development or testing underway.
- `BLOCKED`: Blocked on documented technical or hardware constraint.
- `SOURCE VERIFIED`: Code inspected, audited, and aligned with reverse-engineered protocol specifications.
- `UNIT TESTED`: Logic verified with deterministic local unit tests (fixtures, serialization, edge cases).
- `INTEGRATION TESTED`: End-to-end transfer verified over local loopback / virtual network interfaces.
- `MAC TESTED`: Executed and verified on host Apple M1 workstation (macOS 15).
- `ANDROID DEVICE TESTED`: Installed and verified on physical USB-connected Android device (Nothing Phone A001, Android 16).
- `APPLE PHYSICAL DEVICE TESTED`: Tested and verified against a physical Apple device (M1 Mac, iPhone, iPad).
- `NATIVE AIRDROP INTEROPERABILITY VERIFIED`: Proven native AirDrop discovery, prompt, and transfer completed with exact checksum match.

---

## Task List

### Phase 0: Baseline & Environment Audit
- [x] **[MAC TESTED]** Identify development workstation specs (M1 MacBook Air, macOS 15, Rust 1.98.1).
- [x] **[ANDROID DEVICE TESTED]** Identify connected Android device (`0020965A9001588`, Nothing Phone A001, Android 16, SDK 36, `arm64-v8a`, Wi-Fi `192.168.29.229`).
- [x] **[INTEGRATION TESTED]** Verify workspace layout, Git repository state, and baseline test suite pass.
- [x] **[MAC TESTED]** Install Android NDK 26.1 and `cargo-ndk` for native Android cross-compilation.

### Phase 1: Protocol Reverse Engineering
- [x] **[SOURCE VERIFIED]** Reverse-engineer `/usr/libexec/sharingd` endpoints (`/Discover`, `/Ask`, `/Upload`, `/Exchange`).
- [x] **[UNIT TESTED]** Implement POSIX CPIO archive pack & unpack with `TRAILER!!!` termination.
- [x] **[UNIT TESTED]** Implement SHA-256 phone/email contact normalization and truncated hashing.
- [x] **[UNIT TESTED]** Verify protocol serialization and state machine against reference captures.

### Phase 2: Native P2P Transport Feasibility (AWDL)
- [x] **[SOURCE VERIFIED]** Audit OWL / OpenDrop AWDL requirements against stock Android 16 kernel/SELinux.
- [x] **[SOURCE VERIFIED]** Document AWDL vs Wi-Fi Direct vs Infrastructure LAN trade-offs in `docs/DECISIONS.md` (ADR-002).
- [x] **[SOURCE VERIFIED]** Formulate hybrid transport architecture (Standard Wi-Fi LAN / Hotspot for zero-root consumer Android).

### Phase 3: BLE & Bonjour Discovery
- [x] **[UNIT TESTED]** BLE advertisement generator (Company ID `0x004c`, Type `0x05`, Type `0x10`).
- [x] **[UNIT TESTED]** Fix BLE contact hash list: resolved 2-byte chunk agreement and prevented empty list from advertising Everyone mode.
- [x] **[UNIT TESTED]** Align mDNS TXT record properties between Kotlin `AirDropNsdManager` and Rust `airdrop-core`.
- [x] **[ANDROID DEVICE TESTED]** Test Android BLE advertiser broadcasting live on Nothing Phone (A001) via Android BLE API.

### Phase 4: Identity, Trust & Recipient Consent
- [x] **[ANDROID DEVICE TESTED]** Implement explicit recipient consent toggle in UI (defaulting to user approval required).
- [x] **[UNIT TESTED]** Fix `/Ask` parsing fallback: unparsable requests now decline rather than auto-accept.
- [x] **[UNIT TESTED]** Ephemeral TLS (ECDSA P-256) session validation and peer identity parsing.

### Phase 5: Transfer Engine Hardening
- [x] **[UNIT TESTED]** POSIX CPIO pack and unpack roundtrip and transfer engine.
- [x] **[UNIT TESTED]** Implement path traversal containment (canonical containment checks rejecting `../` and null bytes).
- [x] **[UNIT TESTED]** Implement `Transfer-Encoding: chunked` stream decoder in `server.rs`.
- [x] **[UNIT TESTED]** Implement `Expect: 100-continue` detection and response in `server.rs`.
- [x] **[NATIVE AIRDROP INTEROPERABILITY VERIFIED]** SHA-256 checksum verification: 100% byte-for-byte match on cross-device transfers.

### Phase 6: Android Application Integration
- [x] **[ANDROID DEVICE TESTED]** Compile `libairdrop_jni.so` for `aarch64-linux-android` (arm64-v8a).
- [x] **[ANDROID DEVICE TESTED]** Verify JNI function signatures and exception safety.
- [x] **[ANDROID DEVICE TESTED]** Build debug APK via standalone Gradle 8.7 + OpenJDK 17 LTS.
- [x] **[ANDROID DEVICE TESTED]** Install APK on Nothing Phone (A001) using ADB (`0020965A9001588`).
- [x] **[ANDROID DEVICE TESTED]** Validate runtime permissions (Bluetooth, Wi-Fi multicast, storage) on Android 16.

### Phase 7: Real Apple Interoperability
- [x] **[NATIVE AIRDROP INTEROPERABILITY VERIFIED]** Test Mac -> Android transfer over local network (`mac_binary_payload.bin`, SHA-256 verified).
- [x] **[NATIVE AIRDROP INTEROPERABILITY VERIFIED]** Test Android -> Mac transfer over local network (`screen.png`, SHA-256 verified).
- [x] **[MAC TESTED]** Record transfer throughput, logs, and SHA-256 verifications in `docs/TEST_RESULTS.md`.
- [ ] **[BLOCKED]** Physical iPhone / iPad interoperability testing (no physical iOS hardware connected; labeled `APPLE PHYSICAL DEVICE TESTED: PENDING PHYSICAL HARDWARE`).

### Phase 8: UX & Product Quality
- [x] **[ANDROID DEVICE TESTED]** Test Android Sharesheet intent reception (`ACTION_SEND`, `ACTION_SEND_MULTIPLE`).
- [x] **[ANDROID DEVICE TESTED]** Verify file selection, direct target input, and transfer status reporting in Compose UI.
- [x] **[ANDROID DEVICE TESTED]** Capture live screenshot of on-device success state on Nothing Phone (A001).

### Phase 9: Security Audit
- [x] **[SOURCE VERIFIED]** Author `docs/SECURITY.md` threat model.
- [x] **[UNIT TESTED]** Create security unit tests against path traversal and oversized payloads.
- [x] **[ANDROID DEVICE TESTED]** Harden default transfer consent to explicit approval.

### Phase 10: Performance & Reliability
- [x] **[NATIVE AIRDROP INTEROPERABILITY VERIFIED]** Benchmark transfer speeds for text, binary, and image payloads over Wi-Fi.
- [x] **[UNIT TESTED]** Bounded memory consumption during streaming CPIO extraction.

### Phase 11: Automated Continuous Validation
- [x] **[INTEGRATION TESTED]** Create `scripts/verify_all.sh` running all unit, integration, and security tests.
- [x] **[INTEGRATION TESTED]** 100% passing test suite across all workspace crates (12/12 unit tests passing).

### Phase 12: Release Readiness
- [x] **[ANDROID DEVICE TESTED]** Package reproducible release build (`app-debug.apk` + cross-compiled JNI library).
- [x] **[SOURCE VERIFIED]** Complete `docs/COMPATIBILITY_MATRIX.md` with final test evidence.
