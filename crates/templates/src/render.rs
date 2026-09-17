use serde_json::Value;

use crate::model::{MessageTemplate, RenderedTemplate};

/// Substitutes `{{mixin_key.field}}` references (e.g.
/// `{{core/person@1.0.first_name}}` — consistent with the mixin-key
/// convention used everywhere else, e.g. `context.profile_updates`)
/// against a profile's composed mixins. Hand-rolled rather than a regex
/// dependency — the syntax is simple enough that scanning for `{{`/`}}`
/// pairs is both sufficient and avoids a new crate.
pub fn render(template: &MessageTemplate, mixins: &Value) -> RenderedTemplate {
    let mut missing = Vec::new();
    let body = render_str(&template.body, mixins, &mut missing);
    let subject = template
        .subject
        .as_ref()
        .map(|s| render_str(s, mixins, &mut missing));
    RenderedTemplate {
        subject,
        body,
        missing_variables: missing,
    }
}

fn render_str(text: &str, mixins: &Value, missing: &mut Vec<String>) -> String {
    let mut result = String::with_capacity(text.len());
    let mut rest = text;

    loop {
        let Some(start) = rest.find("{{") else {
            result.push_str(rest);
            break;
        };
        result.push_str(&rest[..start]);
        let after_open = &rest[start + 2..];

        let Some(end) = after_open.find("}}") else {
            // Unmatched "{{" — emit the remainder literally rather than
            // silently dropping it.
            result.push_str(&rest[start..]);
            break;
        };

        let path = after_open[..end].trim();
        match lookup(path, mixins) {
            Some(value) => result.push_str(&value),
            None => missing.push(path.to_string()),
        }
        rest = &after_open[end + 2..];
    }

    result
}

/// `"core/person@1.0.first_name"` -> `mixins["core/person@1.0"]["first_name"]`.
/// Splits at the *last* `.`, since the mixin key itself contains dots
/// (the version, e.g. `1.0`).
fn lookup(path: &str, mixins: &Value) -> Option<String> {
    let (mixin_key, field) = path.rsplit_once('.')?;
    let value = mixins.get(mixin_key)?.get(field)?;
    match value {
        Value::Null => None,
        Value::String(s) => Some(s.clone()),
        other => Some(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json::json;
    use uuid::Uuid;

    fn template(subject: Option<&str>, body: &str) -> MessageTemplate {
        MessageTemplate {
            id: Uuid::nil(),
            tenant_id: Uuid::nil(),
            channel_kind: "email".into(),
            name: "t".into(),
            version: 1,
            subject: subject.map(String::from),
            body: body.into(),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn substitutes_present_variables() {
        let t = template(
            Some("Hi {{core/person@1.0.first_name}}"),
            "Welcome, {{core/person@1.0.first_name}}!",
        );
        let mixins = json!({"core/person@1.0": {"first_name": "Jane"}});
        let rendered = render(&t, &mixins);
        assert_eq!(rendered.subject, Some("Hi Jane".to_string()));
        assert_eq!(rendered.body, "Welcome, Jane!");
        assert!(rendered.missing_variables.is_empty());
    }

    #[test]
    fn reports_missing_variables_as_empty_string() {
        let t = template(
            None,
            "Hi {{core/person@1.0.first_name}}, tier {{core/loyalty@1.0.tier}}",
        );
        let mixins = json!({"core/person@1.0": {"first_name": "Jane"}});
        let rendered = render(&t, &mixins);
        assert_eq!(rendered.body, "Hi Jane, tier ");
        assert_eq!(rendered.missing_variables, vec!["core/loyalty@1.0.tier"]);
    }

    #[test]
    fn handles_multiple_and_adjacent_variables() {
        let t = template(
            None,
            "{{core/person@1.0.first_name}} {{core/person@1.0.last_name}}",
        );
        let mixins = json!({"core/person@1.0": {"first_name": "Jane", "last_name": "Doe"}});
        let rendered = render(&t, &mixins);
        assert_eq!(rendered.body, "Jane Doe");
    }

    #[test]
    fn non_string_values_are_stringified() {
        let t = template(None, "Total: {{core/commerce@1.0.total}}");
        let mixins = json!({"core/commerce@1.0": {"total": 42.5}});
        let rendered = render(&t, &mixins);
        assert_eq!(rendered.body, "Total: 42.5");
    }
}
