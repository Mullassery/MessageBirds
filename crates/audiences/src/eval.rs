use async_recursion::async_recursion;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::model::{AttributeOp, Condition};
use crate::repo::AudienceError;

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

/// Walks a full `Condition` tree — `Attribute` nodes are pure
/// (`eval_attribute_op` above); `Event` nodes query `events` for an
/// occurrence count; `And`/`Or`/`Not` recurse. Shared by the audience
/// engine (`evaluate_and_sync_membership`) and `mb-journeys`' `Condition`
/// nodes, so both use identical evaluation semantics rather than two
/// subtly different copies.
#[async_recursion]
pub async fn evaluate_condition(
    pool: &PgPool,
    condition: &Condition,
    mixins: &Value,
    tenant_id: Uuid,
    profile_id: Uuid,
) -> Result<bool, AudienceError> {
    match condition {
        Condition::Attribute {
            mixin,
            field,
            op,
            value,
        } => {
            let actual = get_field(mixins, mixin, field);
            Ok(eval_attribute_op(*op, actual, value.as_ref()))
        }
        Condition::Event {
            event_type,
            within_days,
            min_count,
        } => {
            let count: i64 = sqlx::query_scalar(
                r#"
                SELECT count(*) FROM events
                WHERE tenant_id = $1 AND profile_id = $2 AND event_type = $3
                  AND occurred_at >= now() - ($4 || ' days')::interval
                "#,
            )
            .bind(tenant_id)
            .bind(profile_id)
            .bind(event_type)
            .bind(within_days.to_string())
            .fetch_one(pool)
            .await?;
            Ok(count >= *min_count)
        }
        Condition::And(conditions) => {
            for c in conditions {
                if !evaluate_condition(pool, c, mixins, tenant_id, profile_id).await? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        Condition::Or(conditions) => {
            for c in conditions {
                if evaluate_condition(pool, c, mixins, tenant_id, profile_id).await? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Condition::Not(inner) => {
            Ok(!evaluate_condition(pool, inner, mixins, tenant_id, profile_id).await?)
        }
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
