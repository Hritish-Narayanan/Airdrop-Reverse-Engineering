# Universal AirDrop Architecture Decision Records (ADRs)

---

## ADR-001: Hybrid Transport Model (Wi-Fi LAN / Hotspot First, AWDL Modularized)

### Status: Accepted
### Context:
Apple AirDrop uses Apple Wireless Direct Link (AWDL), an IEEE 802.11 ad-hoc protocol operating on 802.11 Action Frames with proprietary channel hopping and microsecond-level synchronization.
Stock Android devices (such as the connected Nothing Phone A001 running Android 16 with SELinux `Enforcing`) do not allow unprivileged apps or third-party drivers to inject raw 802.11 frames, disable kernel Wi-Fi firmware power saving, or manipulate low-level MAC timing without kernel-level root and custom Wi-Fi chip firmware (such as OpenWFD or Nexmon on Broadcom chips).
However, Apple's `sharingd` daemon listens on both `awdl0` (IPv6 link-local) AND `en0` (infrastructure Wi-Fi LAN / hotspot) when discoverable.

### Decision:
Implement a hybrid transport architecture:
1. **Primary Operational Path**: Standard Wi-Fi Infrastructure / Mobile Hotspot. This works 100% reliably out-of-the-box on every stock, unrooted Android phone and Mac/iOS device on the same network or phone hotspot without requiring root or firmware modifications.
2. **Modular Transport Abstraction**: The core transport layer is decoupled into an interface `AirDropTransport` so that if an AWDL or Wi-Fi Direct tunnel is established (e.g. on rooted devices or Linux with OWL), the exact same HTTP/TLS/CPIO session layer runs unmodified.

### Consequences:
- **Positive**: 100% compatibility with standard unrooted consumer Android phones (including Android 14, 15, and 16).
- **Positive**: No destructive modifications, no bootloader unlocking, no security compromises.
- **Trade-off**: Requires the Android phone and Apple device to be connected to the same Wi-Fi router OR one device connected to the other's Wi-Fi Hotspot.

---

## ADR-002: Rust Protocol Engine with JNI C-ABI Interface

### Status: Accepted
### Context:
AirDrop requires high-performance crypto (P-256 ECDSA, SHA-256), async networking (tokio, hyper/reqwest), custom TLS validation (rustls), and bit-precise binary archive serialization (POSIX CPIO).
Implementing these low-level protocols purely in Kotlin/Java on Android is error-prone, slow, and lacks portability to Linux/macOS CLI tools.

### Decision:
Implement the entire protocol core in pure Rust (`crates/airdrop-core`), compile to native static/dynamic libraries (`libairdrop_jni.so`), and expose clean C-ABI functions to Kotlin via JNI.

### Consequences:
- **Positive**: Extremely fast transfers, memory safety, shared codebase across CLI and Android app.
- **Positive**: High code testability on development machines without needing Android emulators.
- **Trade-off**: Requires Android NDK cross-compilation toolchain for builds.

---

## ADR-003: POSIX CPIO Archive Serialization (ODC/Newc)

### Status: Accepted
### Context:
Apple AirDrop packages files inside a CPIO container streamed over HTTP POST to `/Upload`. While newer iOS versions experiment with compressed frames (DVZip), the universal fallback supported across all versions of macOS, iOS, and iPadOS is standard POSIX CPIO ending with `TRAILER!!!`.

### Decision:
Implement a high-performance, streaming POSIX CPIO reader and writer directly in Rust that handles multi-file structures, preserves relative filenames, and extracts files directly to disk in chunks.
