# Universal AirDrop Compatibility Matrix

This matrix documents verified, partially verified, and unverified interoperability across devices, operating system versions, and transfer directions.

---

## 1. Directional Interoperability Status

| Direction | Transport | Discovery Method | Transfer Protocol | Status | Evidence |
|---|---|---|---|---|---|
| **Android → Mac** | Local Wi-Fi LAN | mDNS Bonjour (`_airdrop._tcp`) | Ephemeral TLS + CPIO | **VERIFIED (REAL HARDWARE)** | `screen.png` transferred from Nothing Phone A001 to Mac, exact SHA-256 match |
| **Mac → Android** | Local Wi-Fi LAN | BLE (0x004c) + mDNS | Ephemeral TLS + CPIO | **VERIFIED (REAL HARDWARE)** | `mac_binary_payload.bin` transferred from Mac to Nothing Phone A001, exact SHA-256 match |
| **Android → iPhone** | Local Wi-Fi LAN | mDNS Bonjour | Ephemeral TLS + CPIO | **NOT TESTED ON PHYSICAL HARDWARE** | Awaiting physical iPhone test |
| **iPhone → Android** | Local Wi-Fi LAN | BLE (0x004c) + mDNS | Ephemeral TLS + CPIO | **NOT TESTED ON PHYSICAL HARDWARE** | Awaiting physical iPhone test |
| **Android → iPad** | Local Wi-Fi LAN | mDNS Bonjour | Ephemeral TLS + CPIO | **NOT TESTED ON PHYSICAL HARDWARE** | Awaiting physical iPad test |
| **iPad → Android** | Local Wi-Fi LAN | BLE (0x004c) + mDNS | Ephemeral TLS + CPIO | **NOT TESTED ON PHYSICAL HARDWARE** | Awaiting physical iPad test |
| **Apple → Apple** | Native AWDL + Wi-Fi | Native AirDrop | Native Continuity | **PRESERVED** | System `sharingd` unmodified |

---

## 2. Tested Devices & Hardware Inventory

### Workstation / Sender / Receiver
- **Hardware**: Apple MacBook Air (M1, 2020)
- **OS**: macOS Sequoia 15+ (`Darwin 24.x`)
- **IP / Interfaces**: `en0` (`192.168.29.18`), `awdl0` (`fe80::...`)
- **Status**: Verified active and functioning.

### Android Test Device
- **Hardware**: Nothing Phone (Model `A001`, product `GalagaIND`)
- **OS**: Android 16 (API Level 36, Build `B4.1-260928-1137`)
- **Architecture**: `arm64-v8a`
- **SELinux**: `Enforcing`
- **Connection**: USB Debugging via ADB (`0020965A9001588`)
- **Wi-Fi**: `wlan0` (`192.168.29.229/24`)
- **Bluetooth**: Bluetooth LE 5.x with Multiple Advertisement support enabled.
