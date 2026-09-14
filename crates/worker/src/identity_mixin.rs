use mb_core::IdentityRef;
use serde_json::{json, Map, Value};

/// The `core/identity@1.0` mixin's fields are derived automatically from
/// an event's identity claims, not supplied directly by the caller — the
/// identity graph is the source of truth for "who is this", and mixins
/// should reflect it rather than duplicate it.
pub fn derive_identity_mixin_fields(claims: &[IdentityRef]) -> Value {
    let mut fields = Map::new();
    for claim in claims {
        match claim.namespace.as_str() {
            "anonymous_id" => {
                fields.insert("anonymous_id".into(), json!(claim.value));
            }
            "customer_id" => {
                fields.insert("customer_id".into(), json!(claim.value));
            }
            "crm_id" => {
                fields.insert("crm_id".into(), json!(claim.value));
            }
            _ => {}
        }
        if claim.primary {
            fields.insert("primary_namespace".into(), json!(claim.namespace));
        }
    }
    Value::Object(fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claim(namespace: &str, value: &str, primary: bool) -> IdentityRef {
        IdentityRef {
            namespace: namespace.into(),
            value: value.into(),
            primary,
            source: "test".into(),
            confidence: 1.0,
        }
    }

    #[test]
    fn maps_known_namespaces_and_primary_flag() {
        let claims = vec![
            claim("anonymous_id", "a1", false),
            claim("customer_id", "c1", true),
        ];
        let fields = derive_identity_mixin_fields(&claims);
        assert_eq!(fields["anonymous_id"], json!("a1"));
        assert_eq!(fields["customer_id"], json!("c1"));
        assert_eq!(fields["primary_namespace"], json!("customer_id"));
    }

    #[test]
    fn ignores_unmapped_namespaces() {
        let claims = vec![claim("email", "j@example.com", true)];
        let fields = derive_identity_mixin_fields(&claims);
        assert_eq!(fields["primary_namespace"], json!("email"));
        assert!(fields.get("anonymous_id").is_none());
    }
}
