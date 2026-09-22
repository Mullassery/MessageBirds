use sha2::{Digest, Sha256};
use std::env;

/// Environment variable holding the per-deployment pepper mixed into every
/// identity hash. Set this to a random, secret value in every environment
/// that touches real PII (`docker-compose.yml`'s local dev stack is the
/// only place it's OK to leave unset). It must be set to the *same* value
/// for every process that reads or writes `identity_nodes`/`identity_audit`
/// in a given deployment — the hash only remains useful for equality
/// matching if the pepper is consistent, and changing it after data has
/// been written makes every existing hash permanently unmatchable (a
/// re-hash migration would be required). See `SECURITY.md` for the threat
/// this mitigates (precomputed-dictionary attacks against low-entropy
/// values like email/phone) and its limits.
pub const HASH_PEPPER_ENV_VAR: &str = "MB_IDENTITY_HASH_PEPPER";

/// Identity values are stored hashed, never in the clear — the identity
/// graph only needs equality, not the raw email/phone/etc.
///
/// The hash is `SHA-256(pepper || ":" || namespace || ":" || value)` when a
/// pepper is configured via [`HASH_PEPPER_ENV_VAR`]. If that variable is
/// unset (or empty), the pepper is omitted entirely and the hash is
/// byte-for-byte `SHA-256(namespace || ":" || value)` — exactly what this
/// function produced before this change — so deployments that haven't
/// configured a pepper yet keep matching every hash already written to
/// `identity_nodes`/`identity_audit` with no migration. Configuring the
/// pepper for the first time on a deployment with existing data *is* a
/// breaking change (old hashes won't match new ones) and needs a rehash
/// migration; this function intentionally doesn't attempt one.
pub fn hash_value(namespace: &str, value: &str) -> String {
    hash_value_with_pepper(&pepper_from_env(), namespace, value)
}

fn pepper_from_env() -> String {
    env::var(HASH_PEPPER_ENV_VAR).unwrap_or_default()
}

/// Pepper-parameterized core of [`hash_value`], split out so tests can
/// exercise different peppers deterministically without mutating the
/// process environment (which would race with other tests running in
/// parallel in the same binary).
fn hash_value_with_pepper(pepper: &str, namespace: &str, value: &str) -> String {
    let mut hasher = Sha256::new();
    if !pepper.is_empty() {
        hasher.update(pepper.as_bytes());
        hasher.update(b":");
    }
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

    #[test]
    fn same_pepper_and_input_hashes_the_same() {
        assert_eq!(
            hash_value_with_pepper("pepper-a", "email", "a@b.com"),
            hash_value_with_pepper("pepper-a", "email", "a@b.com")
        );
    }

    #[test]
    fn different_pepper_changes_hash() {
        assert_ne!(
            hash_value_with_pepper("pepper-a", "email", "a@b.com"),
            hash_value_with_pepper("pepper-b", "email", "a@b.com")
        );
    }

    #[test]
    fn empty_pepper_matches_unpeppered_legacy_behavior() {
        // Deployments that never set `MB_IDENTITY_HASH_PEPPER` must keep
        // resolving the same hashes they already wrote before this change:
        // byte-for-byte `SHA256(namespace || ":" || value)`, no pepper
        // prefix at all.
        let mut hasher = Sha256::new();
        hasher.update(b"email:a@b.com");
        let legacy_shaped = format!("{:x}", hasher.finalize());
        assert_eq!(
            hash_value_with_pepper("", "email", "a@b.com"),
            legacy_shaped
        );
    }
}
