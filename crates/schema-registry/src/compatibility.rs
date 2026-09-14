use crate::model::Schema;

/// Checks whether `new` is a backward-compatible evolution of `old` (same
/// schema name, new version). We never silently change schema semantics —
/// a breaking change must be rejected with a clear reason, not merged in.
///
/// Breaking:
///   - a field required in `old` is missing in `new`
///   - a field present in both changed type
///   - a newly introduced field is required (old producers wouldn't send it)
///
/// Non-breaking:
///   - a field became optional, or a previously-optional field was dropped
///   - a new optional field was added
///   - enum values were added (loosened)
pub fn check_backward_compatible(old: &Schema, new: &Schema) -> Result<(), Vec<String>> {
    let mut reasons = Vec::new();

    for (name, old_field) in &old.fields {
        match new.fields.get(name) {
            None if old_field.required => {
                reasons.push(format!(
                    "field '{name}' was required in {}@{} and is missing in {}@{}",
                    old.name, old.version, new.name, new.version
                ));
            }
            Some(new_field) if new_field.field_type != old_field.field_type => {
                reasons.push(format!(
                    "field '{name}' changed type from {:?} to {:?}",
                    old_field.field_type, new_field.field_type
                ));
            }
            _ => {}
        }
    }

    for (name, new_field) in &new.fields {
        if new_field.required && !old.fields.contains_key(name) {
            reasons.push(format!(
                "field '{name}' is newly required in {}@{}; old producers wouldn't send it",
                new.name, new.version
            ));
        }
    }

    if reasons.is_empty() {
        Ok(())
    } else {
        Err(reasons)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{FieldDef, FieldType, SchemaStatus};
    use std::collections::BTreeMap;
    use uuid::Uuid;

    fn schema(fields: Vec<(&str, FieldType, bool)>, version: &str) -> Schema {
        let mut map = BTreeMap::new();
        for (name, ty, required) in fields {
            map.insert(
                name.to_string(),
                FieldDef {
                    field_type: ty,
                    required,
                    enum_values: None,
                },
            );
        }
        Schema {
            id: Uuid::new_v4(),
            name: "commerce.product_view".into(),
            version: version.into(),
            fields: map,
            status: SchemaStatus::Active,
            created_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn adding_optional_field_is_compatible() {
        let old = schema(vec![("product_id", FieldType::String, true)], "1.0");
        let new = schema(
            vec![
                ("product_id", FieldType::String, true),
                ("category", FieldType::String, false),
            ],
            "1.1",
        );
        assert!(check_backward_compatible(&old, &new).is_ok());
    }

    #[test]
    fn dropping_required_field_is_breaking() {
        let old = schema(vec![("product_id", FieldType::String, true)], "1.0");
        let new = schema(vec![], "2.0");
        assert!(check_backward_compatible(&old, &new).is_err());
    }

    #[test]
    fn changing_field_type_is_breaking() {
        let old = schema(vec![("price", FieldType::Number, false)], "1.0");
        let new = schema(vec![("price", FieldType::String, false)], "2.0");
        assert!(check_backward_compatible(&old, &new).is_err());
    }

    #[test]
    fn new_required_field_is_breaking() {
        let old = schema(vec![("product_id", FieldType::String, true)], "1.0");
        let new = schema(
            vec![
                ("product_id", FieldType::String, true),
                ("sku", FieldType::String, true),
            ],
            "2.0",
        );
        assert!(check_backward_compatible(&old, &new).is_err());
    }
}
