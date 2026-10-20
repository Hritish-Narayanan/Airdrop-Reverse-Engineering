# Open Questions & Empirical Hypotheses

This document tracks unresolved protocol details, experimental hypotheses, and physical hardware validation items.

---

## 1. Physical Hardware Gap (iOS Devices)

- **Hypothesis**: iOS 17+ and iOS 18 devices will accept incoming AirDrop transfers from our Android implementation over standard Wi-Fi when visibility is set to "Everyone for 10 Minutes".
- **Current Evidence**: Verified on macOS 15 Sequoia (Apple M1); awaiting physical iPhone/iPad hardware testing.
- **Required Experiment**: Connect an iPhone to the same Wi-Fi network, select a photo in the Android AirDrop app, and initiate transfer to the iPhone's advertised mDNS hostname.

---

## 2. DVZip Framing vs Raw CPIO

- **Hypothesis**: For files exceeding 100MB, newer Apple operating systems may negotiate `application/x-dvzip` instead of standard `application/x-cpio`.
- **Current Evidence**: Disassembly of `sharingd` confirms `dvzip` and `application/x-dvzip` presence alongside `application/x-cpio`.
- **Status**: Raw CPIO is fully supported and verified on macOS 15. DVZip streaming support should be added when physical iOS testing requires compressed multi-gigabyte transfers.

---

## 3. Apple ID Identity Services (IDS) Contact Verification

- **Hypothesis**: True "Contacts Only" mode requires an Apple ID certificate signed by Apple's Identity Services CA.
- **Current Evidence**: Third-party non-Apple operating systems cannot obtain signed Apple IDS device certificates without private key extraction or official Apple partnership.
- **Decision**: Document this fundamental boundary honestly in `docs/KNOWN_LIMITATIONS.md`. Interoperability requires "Everyone" mode.
