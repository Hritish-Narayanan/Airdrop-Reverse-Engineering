# Reference Implementations Audit & Licensing Analysis

This document provides a rigorous technical and legal audit of open-source AirDrop and AWDL implementations.

---

## 1. Upstream Implementations Audit

| Project | Language | License | Transport | AirDrop Protocol Scope | Limitations |
|---|---|---|---|---|---|
| **OpenDrop** | Python | GPL-3.0 | AWDL (via OWL) | `/Discover`, `/Ask`, `/Upload` (CPIO) | Python 3, requires root for OWL, Linux/macOS only |
| **OWL** | C | GPL-3.0 | AWDL 802.11 Raw | AWDL Frame Sync & Interface creation | Linux only, requires monitor mode + raw frames |
| **omdrop-owl** | C / Python | GPL-3.0 | AWDL (OWL) | CLI interface for OWL-based AirDrop | Requires root, unmaintained |
| **opendrop-rs** | Rust | GPL-3.0 | AWDL + LAN | Rust AirDrop protocol & AWDL framing | Incomplete Android support, GPL-3.0 obligations |

---

## 2. Licensing Compliance & Clean-Room Architecture

- **License Boundaries**:
  - `seemoo-lab/opendrop` and `opendrop-rs` are licensed under **GPL-3.0**.
  - Direct source copying or linking of GPL-3.0 code into an Android application would impose GPL-3.0 obligations on the entire application package.
- **Clean-Room Implementation**:
  - Our Rust core (`airdrop-core`) and Android integration are written as a **clean-room implementation** based on public protocol specifications, RFCs, and binary reverse-engineering of macOS system frameworks (`sharingd`).
  - No proprietary or third-party GPL code has been copied into this repository.
  - Attribution to the Seemoo Lab academic team is preserved in documentation and research references.
