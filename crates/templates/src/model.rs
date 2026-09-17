use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A versioned, channel-typed message template (Section 29). `subject` is
/// only meaningful for channels that have one (email); `body` always
/// applies. Registering a new version for the same `(tenant, name)`
/// creates a new row with its own id — same pattern as schemas, mixins,
/// and audiences.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MessageTemplate {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub channel_kind: String,
    pub name: String,
    pub version: i32,
    pub subject: Option<String>,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

/// Output of `render` — `missing_variables` lists any `{{mixin.field}}`
/// reference that resolved to nothing (unknown mixin, unset field, or
/// null), rendered as an empty string in `body`/`subject` rather than
/// failing outright, so a partially-populated profile still gets a
/// (imperfect) message instead of none at all.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderedTemplate {
    pub subject: Option<String>,
    pub body: String,
    pub missing_variables: Vec<String>,
}
