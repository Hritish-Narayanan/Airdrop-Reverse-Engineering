use sha2::{Digest, Sha256};

/// Normalizes an email address according to Apple's AirDrop rules:
/// trimmed and converted to lower case ASCII/UTF-8.
pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

/// Normalizes a phone number according to E.164 rules:
/// Strips all spaces, parentheses, hyphens, and keeps leading `+` and digits.
pub fn normalize_phone(phone: &str) -> String {
    let mut normalized = String::new();
    let trimmed = phone.trim();
    for (i, ch) in trimmed.chars().enumerate() {
        if i == 0 && ch == '+' {
            normalized.push(ch);
        } else if ch.is_ascii_digit() {
            normalized.push(ch);
        }
    }
    normalized
}

/// Computes the full 32-byte SHA-256 hash of an identity string (email or phone).
pub fn compute_contact_hash(identity: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(identity.as_bytes());
    hasher.finalize().into()
}

/// Computes a truncated 2-byte or 3-byte contact hash used in BLE AirDrop advertisements.
pub fn compute_truncated_hash(identity: &str, byte_count: usize) -> Vec<u8> {
    let full = compute_contact_hash(identity);
    full[..byte_count.min(32)].to_vec()
}

/// Evaluates if an incoming candidate hash matches any identity in the user's contact list.
pub fn matches_contact(candidate_hash: &[u8], user_identities: &[String]) -> bool {
    for identity in user_identities {
        let full_hash = compute_contact_hash(identity);
        if full_hash.starts_with(candidate_hash) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_normalization() {
        assert_eq!(normalize_email("  Test@Example.COM "), "test@example.com");
    }

    #[test]
    fn test_phone_normalization() {
        assert_eq!(normalize_phone("+1 (415) 555-2671"), "+14155552671");
        assert_eq!(normalize_phone("9876543210"), "9876543210");
    }

    #[test]
    fn test_hash_match() {
        let email = "alice@example.com";
        let truncated = compute_truncated_hash(email, 3);
        assert_eq!(truncated.len(), 3);
        assert!(matches_contact(&truncated, &[email.to_string()]));
        assert!(!matches_contact(&truncated, &["bob@example.com".to_string()]));
    }
}
