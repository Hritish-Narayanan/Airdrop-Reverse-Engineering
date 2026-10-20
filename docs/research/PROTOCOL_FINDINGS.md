# Protocol Reverse Engineering Findings

This document documents verified ground-truth protocol specifications recovered from reverse engineering `/usr/libexec/sharingd` (macOS 15 Sequoia, `arm64e`, SHA-256 `bc10c83aa0d6f197f0c97c92fab0423764073490a411881684a3b17d047f88a7`) using REA and native binary analysis.

---

## 1. Protocol Architecture & Endpoints

AirDrop operates over an HTTPS service advertised via mDNS Bonjour (`_airdrop._tcp.local.`).
The protocol sequence involves three core HTTP/1.1 REST endpoints:

```
Sender                                              Receiver
  |                                                    |
  |--- POST /Discover (Sender Identity & Capabilities)->|
  |<-- 200 OK (Receiver Identity & Capabilities) ------|
  |                                                    |
  |--- POST /Ask (Transfer Metadata & Prompt Request)->|
  |<-- 200 OK (User Consent: Accept / Decline) --------|
  |                                                    |
  |--- POST /Upload (POSIX CPIO Archive Stream) ------>|
  |<-- 200 OK (Extraction Complete & Finalized) -------|
```

### Additional Discovered Endpoints in `sharingd`:
- `POST /Exchange`: Exchange contact cards and Identity Services certificates (used in Contacts Only mode).
- `POST /IdentityShare`: iOS 17+ NameDrop and identity sharing.
- `POST /Error`: Formal error reporting channel.

---

## 2. Recovered Swift Types in `Sharing.framework`

Binary inspection of `/usr/libexec/sharingd` revealed that modern AirDrop employs Swift `Codable` structs:

### `Sharing.SFAirDropReceive.AskRequest`
Recovered fields from native demangled symbols:
- `id`: `TransferIdentifier` (UUID)
- `type`: `TransferType` (Int)
- `transferTypes`: `Int64` bitmask
- `canAutoAccept`: `Bool`
- `verifiableIdentity`: `Bool`
- `senderIsMe`: `Bool`
- `contactIdentifier`: `String?`
- `senderBundleID`: `String` (e.g. `com.apple.finder`, `com.airdrop.android`)
- `senderComputerName`: `String` (advertised host name)
- `senderModelName`: `String` (e.g. `MacBookAir`, `Pixel`)
- `senderDisplayName`: `String` (human-readable sender name)
- `senderEmail`: `String?`
- `senderEmailHash`: `String?` (SHA-256 truncated hash)
- `senderFirstName`: `String?`
- `senderLastName`: `String?`
- `senderCompositeName`: `String?`
- `itemsDescription`: `String?` (e.g. "1 photo", "3 files")
- `items`: `Set<ItemInfo>`
- `urlItems`: `Set<URL>`
- `customPayload`: `Data?`
- `pairingPin`: `String?`

### `Sharing.SFAirDropReceive.AskResponse`
Enumeration cases recovered from symbols:
1. `accept(ItemDestination, AcceptContext)`: Recipient consented to receive the transfer.
2. `decline`: Recipient rejected the transfer.
3. `mixedTypes`: Error when items cannot be simultaneously accepted.
4. `error`: General processing failure.

---

## 3. Framing & MIME Types

Binary analysis of `sharingd` revealed the following exact content types and archiving routines:

| Identifier | Format | Usage | Status |
|---|---|---|---|
| `application/x-cpio` | POSIX CPIO (`newc` / `odc`) | Standard multi-file payload archive | **Implemented & Verified** |
| `application/x-dvzip` | Framed compression wrapper | iOS 16+ / macOS 13+ large stream compression | Supported in modern macOS |
| `TRAILER!!!` | ASCII boundary marker | POSIX CPIO end-of-archive record | **Implemented & Verified** |

### Path Traversal Defense:
All CPIO entry filenames must be strictly validated against canonical target directory containment. Any entry containing null bytes (`\0`) or path separators escaping the download directory must be rejected.

---

## 4. Multi-Format Serialization (bplist00, XML Plist, JSON)

Apple operating systems utilize different serialization formats across major releases:
- **Legacy iOS / macOS (iOS 7–12, OS X Yosemite–Catalina)**: Exclusively encode `/Ask` requests as Apple Binary Property Lists (`bplist00`).
- **Modern iOS / macOS (iOS 13–18, macOS Big Sur–Sequoia)**: Primarily encode as JSON or `bplist00` depending on whether Swift `Codable` or Objective-C `NSPropertyListSerialization` path is invoked.

### Implementation Solution:
Our receiver introduces [`parse_airdrop_payload`](file:///Users/hritish/Documents/REA/crates/airdrop-core/src/server.rs):
1. Inspects leading magic bytes:
   - If payload begins with `bplist00`, it deserializes via `plist::from_bytes`.
   - If payload begins with `<?xml` or `<!DOCTYPE plist`, it deserializes via XML plist parser.
   - Otherwise, it deserializes as standard JSON via `serde_json::from_slice`.
2. Tested and verified against synthetic binary plists generated via `plist::to_writer_binary`, XML plists, and JSON dictionaries in `server::tests::test_parse_airdrop_payload_*`.

