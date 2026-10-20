# Transport Feasibility & Wireless Architecture

This document evaluates the feasibility of native peer-to-peer transport (Apple Wireless Direct Link / AWDL) versus Wi-Fi Infrastructure LAN / Mobile Hotspot for Universal AirDrop on Android.

---

## 1. What is Apple Wireless Direct Link (AWDL)?

AWDL is Apple's proprietary peer-to-peer Wi-Fi protocol operating over 802.11 action frames. It features:
- Synchronization via Action Frame sequences transmitted on fixed channels (Channel 6 or Channel 44).
- Dedicated virtual network interface (`awdl0`) assigned an IPv6 link-local address (`fe80::...%awdl0`).
- Election algorithm to designate a master node for master indication frames (MIF).
- Slot-based time-sharing between the connected infrastructure BSSID and the AWDL channel.

---

## 2. Android Hardware & OS Constraints (Stock Android 16)

Investigation of the connected test device (**Nothing Phone A001**, Android 16, SDK 36):

1. **SELinux Enforcing**:
   - Android 16 enforces strict SELinux domain isolation for unprivileged apps (`u:r:untrusted_app:s0`).
   - Raw socket creation (`AF_PACKET`, `SOCK_RAW`) requires `CAP_NET_RAW`, which is denied to untrusted apps.
2. **Wi-Fi Driver Boundaries**:
   - Stock Android Wi-Fi HAL (`wpa_supplicant` / `nl80211`) does not expose AWDL-compatible channel-hopping action frame interfaces to userspace.
   - Monitor mode and custom 802.11 frame injection require vendor driver modifications and root privileges.
3. **Wi-Fi Multicast Filtering**:
   - Android Wi-Fi drivers drop multicast packets by default to save power.
   - Apps must explicitly acquire `android.net.wifi.WifiManager.MulticastLock` to receive mDNS Bonjour discovery packets from other devices on the network.

---

## 3. Dual-Transport Hybrid Solution

To achieve universal compatibility without requiring users to root their devices, our architecture implements:

```
+----------------------------------------------------------------------+
|                     Universal AirDrop Layer                          |
|  - Ephemeral TLS P-256                                               |
|  - HTTP/1.1 REST (/Discover, /Ask, /Upload)                          |
|  - POSIX CPIO Streaming Extraction                                   |
+----------------------------------------------------------------------+
           |                                           |
           v                                           v
[Zero-Root Wi-Fi Transport]                 [Experimental AWDL Engine]
- Standard 802.11 Infrastructure LAN         - Requires Root + Monitor Driver
- Android Wi-Fi Hotspot (Direct P2P)         - Linux OWL / OpenDrop port
- Supported on ALL Android 8.0+ devices      - Custom kernel / firmware only
- Apple sharingd listens on en0!
```

### Key Breakthrough: Apple `sharingd` Listens on `en0`
Reverse engineering and runtime verification confirm that Apple's AirDrop daemon (`sharingd`) listens for HTTP and mDNS traffic on **both `awdl0` and `en0` (standard Wi-Fi)** when the device is on a Wi-Fi network with AirDrop set to "Everyone". This allows zero-root consumer Android devices to exchange files seamlessly with Macs, iPhones, and iPads over local Wi-Fi or mobile hotspot.
