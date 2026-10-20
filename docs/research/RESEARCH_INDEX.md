# Universal AirDrop Research Index

This directory contains persistent, evidence-backed reverse engineering findings, protocol specifications, reference implementation audits, and hardware feasibility evaluations for Universal AirDrop (Android ↔ Apple).

---

## Document Map

| Document | Scope | Status | Primary Sources |
|---|---|---|---|
| [`PROTOCOL_FINDINGS.md`](file:///Users/hritish/Documents/REA/docs/research/PROTOCOL_FINDINGS.md) | Disassembled `sharingd` endpoints, Swift Codable structs, MIME types, and archive framing | **VERIFIED** | macOS 15 `/usr/libexec/sharingd`, REA tool inspection, live packet traces |
| [`TRANSPORT_FEASIBILITY.md`](file:///Users/hritish/Documents/REA/docs/research/TRANSPORT_FEASIBILITY.md) | AWDL vs Wi-Fi LAN / Hotspot analysis, kernel/SELinux constraints on Android 16 | **VERIFIED** | Nothing Phone A001 diagnostics, OWL/OpenDrop papers, USENIX 2019 |
| [`REFERENCE_IMPLEMENTATIONS.md`](file:///Users/hritish/Documents/REA/docs/research/REFERENCE_IMPLEMENTATIONS.md) | OpenDrop, OWL, omdrop-owl, opendrop-rs audits, licensing, and limitations | **VERIFIED** | GitHub upstream codebases, licenses, USENIX Security 2019 |
| [`OPEN_QUESTIONS.md`](file:///Users/hritish/Documents/REA/docs/research/OPEN_QUESTIONS.md) | Known protocol uncertainties, physical hardware gaps, and next validation experiments | **ACTIVE** | Empirical test runs, ADB logs |

---

## Methodology & Evidence Hierarchy

1. **Level 1 (Direct Binary Disassembly)**: Mach-O inspection via REA and native symbol dump on `/usr/libexec/sharingd` (macOS 15 Sequoia).
2. **Level 2 (Hardware-Verified Captures)**: End-to-end transfers between Apple M1 Mac and physical Nothing Phone A001 with SHA-256 validation.
3. **Level 3 (Peer-Reviewed Academic Research)**: USENIX Security 2019 (*A Billion Open Interfaces for Eve and Mallory*).
4. **Level 4 (Open-Source Reference Implementations)**: OpenDrop (Python), OWL (C), opendrop-rs (Rust).
