# Universal AirDrop Engineering Roadmap

This roadmap tracks all project phases, dependencies, technical blockers, and strict exit gates.

---

## Phase Matrix

| Phase | Title | Dependencies | Status | Primary Output |
|---|---|---|---|---|
| **Phase 0** | Baseline & Environment Audit | None | **VERIFIED** | Hardware & Toolchain inventory (`AGENTS.md`) |
| **Phase 1** | Protocol Reverse Engineering | Phase 0 | **VERIFIED** | `docs/PROTOCOL_SPEC.md`, `airdrop_protocol_spec.md` |
| **Phase 2** | Native P2P Transport Feasibility | Phase 1 | **IN PROGRESS** | AWDL on Android feasibility report (`docs/DECISIONS.md`) |
| **Phase 3** | BLE & Bonjour Discovery Audit | Phase 1 | **IN PROGRESS** | Contact hash fix, BLE advertisement tests |
| **Phase 4** | Identity, Trust & Recipient Consent | Phase 1, 3 | **NOT STARTED** | Explicit recipient consent flow, TLS verifier |
| **Phase 5** | Transfer Engine Hardening | Phase 1, 4 | **IN PROGRESS** | Streaming CPIO, path traversal protection, MIME/UTI |
| **Phase 6** | Android Application Integration | Phase 3, 4, 5 | **IN PROGRESS** | JNI bridge, APK build, ADB install & smoke test |
| **Phase 7** | Real Apple Interoperability | Phase 2, 5, 6 | **NOT STARTED** | macOS ↔ Android & iOS ↔ Android tests |
| **Phase 8** | UX & Product Quality | Phase 6 | **NOT STARTED** | Jetpack Compose UI, Sharesheet `ACTION_SEND` |
| **Phase 9** | Security Audit & Threat Model | Phase 4, 5 | **NOT STARTED** | `docs/SECURITY.md`, fuzz tests, path containment |
| **Phase 10** | Performance & Reliability | Phase 5, 6 | **NOT STARTED** | Bounded memory streaming, benchmark scripts |
| **Phase 11** | Automated Continuous Validation | Phase 0–10 | **NOT STARTED** | Unified verification script `scripts/verify_all.sh` |
| **Phase 12** | Release Readiness | Phase 11 | **NOT STARTED** | Release APK, reproducible builds, release notes |

---

## Detailed Phase Acceptance Criteria

### Phase 0: Baseline & Environment Audit
- [x] Identify dev machine architecture (M1 Apple Silicon, macOS 15+).
- [x] Identify connected Android device via ADB (`Nothing A001`, Android 16, SDK 36, `arm64-v8a`, `192.168.29.229`).
- [x] Verify toolchain (`rustc 1.98.1`, OpenJDK 26, Android SDK 34, Android Commandline Tools).
- [x] Verify baseline repo cleanliness and test execution.

### Phase 1: Protocol Reverse Engineering
- [x] Extract `sharingd` endpoints: `/Discover`, `/Ask`, `/Upload`.
- [x] Document POSIX CPIO archive framing and `TRAILER!!!` structure.
- [x] Define JSON manifest dictionary keys (`SenderComputerName`, `SenderModelName`, `Files`).
- [ ] Document bplist vs JSON edge cases across older/newer iOS versions.

### Phase 2: Native P2P Transport Feasibility (AWDL)
- [ ] Determine whether stock unrooted Android 16 can frame or inject 802.11 Action Frames for AWDL.
- [ ] Document exact hardware/driver requirements (Broadcom/Qualcomm Wi-Fi Direct vs AWDL timing synchronization).
- [ ] Establish hybrid model: Standard Wi-Fi Infrastructure (zero-root, zero-config on same network/hotspot) + AWDL/Wi-Fi Direct feasibility boundaries.

### Phase 3: BLE & Bonjour Discovery
- [ ] Audit BLE manufacturer data format (`0x004c`, Type `0x05`, Type `0x10`).
- [ ] Fix empty contact hash edge case (do not silently switch between Contacts Only and Everyone).
- [ ] Verify mDNS IPv4 and IPv6 link-local handling.
- [ ] Real-device BLE broadcast test using Android `BluetoothLeAdvertiser`.

### Phase 4: Identity, Trust & Recipient Consent
- [ ] Remove any dangerous auto-accept defaults in receiver mode.
- [ ] Implement explicit approval state machine (`Pending` -> `Accepted` / `Declined`).
- [ ] Strict certificate validation policy (pinning or contact hash validation).

### Phase 5: Transfer Engine Hardening
- [ ] Stream files with bounded memory buffers (never load entire large files into RAM).
- [ ] Enforce strict path traversal prevention (`../` stripping and canonicalization).
- [ ] SHA-256 integrity verification during and after transfer.
- [ ] Bounded archive decompression and symlink escape rejection.

### Phase 6: Android App Integration
- [ ] Install Android NDK 26.1 and `cargo-ndk`.
- [ ] Compile `libairdrop_jni.so` for `arm64-v8a`.
- [ ] Build debug APK using Gradle.
- [ ] Install APK onto Nothing Phone (A001) via ADB.
- [ ] Test real-device lifecycle, permissions, and network discovery.

### Phase 7: Real Apple Interoperability
- [ ] Test Mac (M1) -> Android (Nothing Phone) transfer over AirDrop protocol.
- [ ] Test Android -> Mac transfer.
- [ ] Record exact transfer logs, speeds, and file hash verifications in `docs/TEST_RESULTS.md`.

### Phase 8: UX & Product Quality
- [ ] Test Android system Sharesheet integration (`ACTION_SEND`).
- [ ] Display real-time progress bars, transfer speeds, and device model icons.
- [ ] Proper error messaging on decline or disconnect.

### Phase 9: Security Audit
- [ ] Threat model document in `docs/SECURITY.md`.
- [ ] Path traversal test fixtures.
- [ ] Memory bounds verification under heavy file transfers.
