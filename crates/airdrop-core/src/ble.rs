pub const APPLE_COMPANY_ID: u16 = 0x004C;

pub const TYPE_AIRDROP: u8 = 0x05;
pub const TYPE_NEARBY_ACTION: u8 = 0x10;
pub const ACTION_AIRDROP: u8 = 0x01;

/// Represents an AirDrop BLE advertisement payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AirDropBlePayload {
    /// Traditional AirDrop advertisement (Type 0x05).
    AirDrop {
        discoverable_by_everyone: bool,
        contact_hashes: Vec<Vec<u8>>,
    },
    /// Modern Nearby Action advertisement (Type 0x10).
    NearbyAction {
        action_type: u8,
        flags: u8,
        auth_tag: Vec<u8>,
    },
}

impl AirDropBlePayload {
    /// Builds a Type 0x05 AirDrop advertisement packet for "Everyone" mode.
    pub fn new_everyone() -> Self {
        AirDropBlePayload::AirDrop {
            discoverable_by_everyone: true,
            contact_hashes: Vec::new(),
        }
    }

    /// Builds a Type 0x05 AirDrop advertisement packet with contact hashes.
    pub fn new_contacts(hashes: Vec<Vec<u8>>) -> Self {
        AirDropBlePayload::AirDrop {
            discoverable_by_everyone: false,
            contact_hashes: hashes,
        }
    }

    /// Builds a Type 0x10 NearbyAction advertisement packet (AirDrop proximity beacon).
    pub fn new_nearby_action(flags: u8) -> Self {
        AirDropBlePayload::NearbyAction {
            action_type: ACTION_AIRDROP,
            flags,
            auth_tag: Vec::new(),
        }
    }

    /// Encodes this payload into raw manufacturer data (excluding the 2-byte company ID).
    pub fn encode(&self) -> Vec<u8> {
        match self {
            AirDropBlePayload::AirDrop {
                discoverable_by_everyone,
                contact_hashes,
            } => {
                let mut data = Vec::with_capacity(18);
                data.push(TYPE_AIRDROP);

                let mut body = Vec::new();
                // 8 bytes of zeros / state flags
                body.extend_from_slice(&[0u8; 8]);

                if *discoverable_by_everyone {
                    // In Everyone mode, 8 bytes of zeroes for hashes
                    body.extend_from_slice(&[0u8; 8]);
                } else if contact_hashes.is_empty() {
                    // In Contacts Only mode with empty contacts, emit non-zero padding (never advertise Everyone)
                    body.extend_from_slice(&[0xFFu8; 8]);
                } else {
                    for hash in contact_hashes {
                        if hash.len() >= 2 {
                            body.extend_from_slice(&hash[..2]);
                        } else if !hash.is_empty() {
                            body.push(hash[0]);
                            body.push(0);
                        }
                    }
                    // Pad to 16 bytes body if needed
                    while body.len() < 16 {
                        body.push(0);
                    }
                }

                data.push(body.len() as u8);
                data.extend_from_slice(&body);
                data
            }
            AirDropBlePayload::NearbyAction {
                action_type,
                flags,
                auth_tag,
            } => {
                let mut data = Vec::new();
                data.push(TYPE_NEARBY_ACTION);
                let length = 2 + auth_tag.len() as u8;
                data.push(length);
                data.push(*flags);
                data.push(*action_type);
                data.extend_from_slice(auth_tag);
                data
            }
        }
    }

    /// Parses raw manufacturer data (starting after the 2-byte company ID 0x004c).
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 2 {
            return None;
        }

        let type_id = data[0];
        let len = data[1] as usize;

        if data.len() < 2 + len {
            return None;
        }

        let body = &data[2..2 + len];

        match type_id {
            TYPE_AIRDROP => {
                if body.len() < 8 {
                    return None;
                }
                // Check if hashes are all zeros
                let hashes_part = if body.len() >= 16 { &body[8..16] } else { &[] };
                let is_all_zero = hashes_part.iter().all(|&b| b == 0);

                let contact_hashes = if is_all_zero || hashes_part.is_empty() {
                    Vec::new()
                } else {
                    hashes_part
                        .chunks(2)
                        .filter(|c| c != &[0xff, 0xff] && c != &[0x00, 0x00])
                        .map(|c| c.to_vec())
                        .collect()
                };

                Some(AirDropBlePayload::AirDrop {
                    discoverable_by_everyone: is_all_zero,
                    contact_hashes,
                })
            }
            TYPE_NEARBY_ACTION => {
                if body.len() < 2 {
                    return None;
                }
                let flags = body[0];
                let action_type = body[1];
                let auth_tag = body[2..].to_vec();

                Some(AirDropBlePayload::NearbyAction {
                    action_type,
                    flags,
                    auth_tag,
                })
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ble_everyone_encode_parse() {
        let payload = AirDropBlePayload::new_everyone();
        let encoded = payload.encode();

        assert_eq!(encoded[0], TYPE_AIRDROP);
        assert_eq!(encoded[1], 16); // length of body

        let parsed = AirDropBlePayload::parse(&encoded).expect("parse failed");
        assert_eq!(parsed, payload);
    }

    #[test]
    fn test_ble_contacts_mode_never_everyone() {
        // Empty contacts list must NOT encode as Everyone
        let payload = AirDropBlePayload::new_contacts(vec![]);
        let encoded = payload.encode();
        let parsed = AirDropBlePayload::parse(&encoded).expect("parse failed");
        match parsed {
            AirDropBlePayload::AirDrop { discoverable_by_everyone, contact_hashes } => {
                assert!(!discoverable_by_everyone, "Empty contacts mode must NOT be discoverable by everyone");
                assert!(contact_hashes.is_empty());
            }
            _ => panic!("Expected AirDrop variant"),
        }

        // With actual contact hashes
        let payload_with_hash = AirDropBlePayload::new_contacts(vec![vec![0x12, 0x34], vec![0x56, 0x78]]);
        let enc2 = payload_with_hash.encode();
        let parsed2 = AirDropBlePayload::parse(&enc2).expect("parse failed");
        match parsed2 {
            AirDropBlePayload::AirDrop { discoverable_by_everyone, contact_hashes } => {
                assert!(!discoverable_by_everyone);
                assert_eq!(contact_hashes.len(), 2);
                assert_eq!(contact_hashes[0], vec![0x12, 0x34]);
                assert_eq!(contact_hashes[1], vec![0x56, 0x78]);
            }
            _ => panic!("Expected AirDrop variant"),
        }
    }
}
