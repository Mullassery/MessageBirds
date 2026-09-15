use serde_json::Value;

use crate::model::AttributeOp;

/// Reads `mixins[mixin][field]` out of a composed profile's mixins JSON
/// (`{"core/person@1.0": {"first_name": "Jane"}, ...}`).
pub fn get_field<'a>(mixins: &'a Value, mixin: &str, field: &str) -> Option<&'a Value> {
    mixins.get(mixin)?.get(field)
}

/// Pure comparison logic for one `Condition::Attribute` node — no DB, no
/// async, fully unit-testable. `actual` is what's on the profile,
/// `expected` is the condition's configured `value` (absent for
/// `Exists`/`NotExists`, which don't need one).
pub fn eval_attribute_op(
    op: AttributeOp,
    actual: Option<&Value>,
    expected: Option<&Value>,
) -> bool {
    let present = actual.is_some_and(|v| !v.is_null());

    match op {
        AttributeOp::Exists => present,
        AttributeOp::NotExists => !present,
        AttributeOp::Equals => match (actual, expected) {
            (Some(a), Some(e)) => a == e,
            _ => false,
        },
        AttributeOp::NotEquals => match (actual, expected) {
            (Some(a), Some(e)) => a != e,
            (None, Some(_)) => true,
            _ => false,
        },
        AttributeOp::GreaterThan => numeric(actual, expected, |a, e| a > e),
        AttributeOp::LessThan => numeric(actual, expected, |a, e| a < e),
        AttributeOp::Contains => match (actual, expected) {
            (Some(Value::String(a)), Some(Value::String(e))) => a.contains(e.as_str()),
            (Some(Value::Array(a)), Some(e)) => a.contains(e),
            _ => false,
        },
    }
}

fn numeric(
    actual: Option<&Value>,
    expected: Option<&Value>,
    cmp: impl Fn(f64, f64) -> bool,
) -> bool {
    match (
        actual.and_then(Value::as_f64),
        expected.and_then(Value::as_f64),
    ) {
        (Some(a), Some(e)) => cmp(a, e),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn exists_and_not_exists() {
        assert!(eval_attribute_op(
            AttributeOp::Exists,
            Some(&json!("x")),
            None
        ));
        assert!(!eval_attribute_op(AttributeOp::Exists, None, None));
        assert!(!eval_attribute_op(
            AttributeOp::Exists,
            Some(&json!(null)),
            None
        ));
        assert!(eval_attribute_op(AttributeOp::NotExists, None, None));
        assert!(!eval_attribute_op(
            AttributeOp::NotExists,
            Some(&json!("x")),
            None
        ));
    }

    #[test]
    fn equals_and_not_equals() {
        assert!(eval_attribute_op(
            AttributeOp::Equals,
            Some(&json!("a")),
            Some(&json!("a"))
        ));
        assert!(!eval_attribute_op(
            AttributeOp::Equals,
            Some(&json!("a")),
            Some(&json!("b"))
        ));
        assert!(!eval_attribute_op(
            AttributeOp::Equals,
            None,
            Some(&json!("a"))
        ));

        assert!(eval_attribute_op(
            AttributeOp::NotEquals,
            Some(&json!("a")),
            Some(&json!("b"))
        ));
        assert!(eval_attribute_op(
            AttributeOp::NotEquals,
            None,
            Some(&json!("a"))
        ));
        assert!(!eval_attribute_op(
            AttributeOp::NotEquals,
            Some(&json!("a")),
            Some(&json!("a"))
        ));
    }

    #[test]
    fn greater_than_and_less_than() {
        assert!(eval_attribute_op(
            AttributeOp::GreaterThan,
            Some(&json!(10)),
            Some(&json!(5))
        ));
        assert!(!eval_attribute_op(
            AttributeOp::GreaterThan,
            Some(&json!(5)),
            Some(&json!(10))
        ));
        assert!(eval_attribute_op(
            AttributeOp::LessThan,
            Some(&json!(5)),
            Some(&json!(10))
        ));
        assert!(!eval_attribute_op(
            AttributeOp::GreaterThan,
            Some(&json!("not a number")),
            Some(&json!(5))
        ));
    }

    #[test]
    fn contains_on_strings_and_arrays() {
        assert!(eval_attribute_op(
            AttributeOp::Contains,
            Some(&json!("hello world")),
            Some(&json!("world"))
        ));
        assert!(!eval_attribute_op(
            AttributeOp::Contains,
            Some(&json!("hello world")),
            Some(&json!("bye"))
        ));
        assert!(eval_attribute_op(
            AttributeOp::Contains,
            Some(&json!(["a", "b", "c"])),
            Some(&json!("b"))
        ));
    }

    #[test]
    fn get_field_reads_nested_mixin_value() {
        let mixins = json!({"core/person@1.0": {"first_name": "Jane"}});
        assert_eq!(
            get_field(&mixins, "core/person@1.0", "first_name"),
            Some(&json!("Jane"))
        );
        assert_eq!(get_field(&mixins, "core/person@1.0", "last_name"), None);
        assert_eq!(get_field(&mixins, "core/contact@1.0", "email"), None);
    }
}
