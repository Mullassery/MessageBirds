use std::collections::BTreeMap;

use crate::model::{FieldDef, Schema};

#[derive(Debug, Clone, PartialEq)]
pub enum FieldViolation {
    Missing {
        field: String,
    },
    WrongType {
        field: String,
        expected: &'static str,
    },
    NotInEnum {
        field: String,
    },
}

impl std::fmt::Display for FieldViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldViolation::Missing { field } => {
                write!(f, "field '{field}' is required but missing")
            }
            FieldViolation::WrongType { field, expected } => {
                write!(f, "field '{field}' must be of type {expected}")
            }
            FieldViolation::NotInEnum { field } => {
                write!(f, "field '{field}' is not one of the allowed values")
            }
        }
    }
}

/// Validates an event's `data` payload against its schema's field
/// definitions. Returns every violation found, not just the first, so the
/// caller (the worker's DLQ path) can report a complete reason.
pub fn validate_payload(schema: &Schema, data: &serde_json::Value) -> Vec<FieldViolation> {
    validate_fields(&schema.fields, data)
}

/// Field-level validation shared by schemas (event payloads) and mixins
/// (profile field composition) — both are "a map of field name to
/// definition, checked against a JSON object".
pub fn validate_fields(
    fields: &BTreeMap<String, FieldDef>,
    data: &serde_json::Value,
) -> Vec<FieldViolation> {
    let mut violations = Vec::new();
    let obj = data.as_object();

    for (field_name, field_def) in fields {
        let value = obj.and_then(|o| o.get(field_name));
        match value {
            None => {
                if field_def.required {
                    violations.push(FieldViolation::Missing {
                        field: field_name.clone(),
                    });
                }
            }
            Some(v) if v.is_null() => {
                if field_def.required {
                    violations.push(FieldViolation::Missing {
                        field: field_name.clone(),
                    });
                }
            }
            Some(v) => {
                if !field_def.field_type.matches(v) {
                    violations.push(FieldViolation::WrongType {
                        field: field_name.clone(),
                        expected: type_name(field_def.field_type),
                    });
                    continue;
                }
                if let Some(allowed) = &field_def.enum_values {
                    if !allowed.contains(v) {
                        violations.push(FieldViolation::NotInEnum {
                            field: field_name.clone(),
                        });
                    }
                }
            }
        }
    }

    violations
}

fn type_name(t: crate::model::FieldType) -> &'static str {
    use crate::model::FieldType::*;
    match t {
        String => "string",
        Number => "number",
        Boolean => "boolean",
        Object => "object",
        Array => "array",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{FieldDef, FieldType, SchemaStatus};
    use std::collections::BTreeMap;
    use uuid::Uuid;

    fn product_view_schema() -> Schema {
        let mut fields = BTreeMap::new();
        fields.insert(
            "product_id".to_string(),
            FieldDef {
                field_type: FieldType::String,
                required: true,
                enum_values: None,
            },
        );
        fields.insert(
            "price".to_string(),
            FieldDef {
                field_type: FieldType::Number,
                required: false,
                enum_values: None,
            },
        );
        Schema {
            id: Uuid::new_v4(),
            name: "commerce.product_view".into(),
            version: "1.0".into(),
            fields,
            status: SchemaStatus::Active,
            created_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn valid_payload_has_no_violations() {
        let schema = product_view_schema();
        let data = serde_json::json!({"product_id": "p1", "price": 9.99});
        assert!(validate_payload(&schema, &data).is_empty());
    }

    #[test]
    fn missing_required_field_is_reported() {
        let schema = product_view_schema();
        let data = serde_json::json!({"price": 9.99});
        let violations = validate_payload(&schema, &data);
        assert_eq!(
            violations,
            vec![FieldViolation::Missing {
                field: "product_id".into()
            }]
        );
    }

    #[test]
    fn wrong_type_is_reported() {
        let schema = product_view_schema();
        let data = serde_json::json!({"product_id": "p1", "price": "not-a-number"});
        let violations = validate_payload(&schema, &data);
        assert_eq!(
            violations,
            vec![FieldViolation::WrongType {
                field: "price".into(),
                expected: "number"
            }]
        );
    }
}
