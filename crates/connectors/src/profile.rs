//! Small, dependency-free extraction of the standard profile fields that
//! CRM/marketing destinations need from a `mixins` payload.

use serde_json::Value;

pub(crate) fn find_string(value: &Value, keys: &[&str]) -> Option<String> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(Value::String(found)) = map.get(*key) {
                    if !found.trim().is_empty() {
                        return Some(found.clone());
                    }
                }
            }
            for nested in map.values() {
                if let Some(found) = find_string(nested, keys) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(items) => items.iter().find_map(|item| find_string(item, keys)),
        _ => None,
    }
}

pub(crate) fn email(payload: &Value) -> Option<String> {
    find_string(payload, &["email", "email_address"])
}

pub(crate) fn first_name(payload: &Value) -> Option<String> {
    find_string(payload, &["first_name", "firstname", "given_name"])
}

pub(crate) fn last_name(payload: &Value) -> Option<String> {
    find_string(payload, &["last_name", "lastname", "family_name"])
}

pub(crate) fn phone(payload: &Value) -> Option<String> {
    find_string(payload, &["phone", "phone_number", "mobile"])
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extracts_standard_mixin_fields() {
        let payload = json!({
            "mixins": {
                "core/person@1.0": {"first_name": "Ada", "last_name": "Lovelace"},
                "core/contact@1.0": {"email": "ada@example.test", "phone": "+15550100"}
            }
        });
        assert_eq!(email(&payload).as_deref(), Some("ada@example.test"));
        assert_eq!(first_name(&payload).as_deref(), Some("Ada"));
        assert_eq!(last_name(&payload).as_deref(), Some("Lovelace"));
        assert_eq!(phone(&payload).as_deref(), Some("+15550100"));
    }

    #[test]
    fn missing_or_empty_values_are_none() {
        let payload = json!({"mixins": {"core/person@1.0": {"first_name": "  "}}});
        assert_eq!(email(&payload), None);
        assert_eq!(first_name(&payload), None);
    }
}
