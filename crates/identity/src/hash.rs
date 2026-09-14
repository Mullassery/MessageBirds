use sha2::{Digest, Sha256};

/// Identity values are stored hashed, never in the clear — the identity
/// graph only needs equality, not the raw email/phone/etc.
pub fn hash_value(namespace: &str, value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(namespace.as_bytes());
    hasher.update(b":");
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_input_hashes_the_same() {
        assert_eq!(
            hash_value("email", "a@b.com"),
            hash_value("email", "a@b.com")
        );
    }

    #[test]
    fn different_namespace_changes_hash() {
        assert_ne!(hash_value("email", "x"), hash_value("phone", "x"));
    }
}
