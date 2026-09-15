use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttributeOp {
    Equals,
    NotEquals,
    Exists,
    NotExists,
    GreaterThan,
    LessThan,
    Contains,
}

/// An audience's membership rule. `Attribute` compares a mixin field on the
/// profile being evaluated; `Event` checks how many times an event type
/// occurred for that profile within a trailing window. `And`/`Or`/`Not`
/// compose these. There is deliberately no `Sequence` variant — ordered,
/// time-aware event chains ("A then B then not C") are a different
/// evaluation model and not implemented here.
///
/// Externally tagged on the wire — `{"attribute": {...}}`, not
/// `{"type": "attribute", ...}` — because internally-tagged
/// (`#[serde(tag = "type")]`) enums use serde's `Content`-buffering
/// strategy, which blows the compiler's type-instantiation limit on a
/// recursive type like this one (`Not(Box<Condition>)`). Not a style
/// choice; the tagged form doesn't compile here.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Condition {
    Attribute {
        /// e.g. `"core/person@1.0"`
        mixin: String,
        field: String,
        op: AttributeOp,
        #[serde(default)]
        value: Option<serde_json::Value>,
    },
    Event {
        event_type: String,
        within_days: i64,
        #[serde(default = "default_min_count")]
        min_count: i64,
    },
    And(Vec<Condition>),
    Or(Vec<Condition>),
    Not(Box<Condition>),
}

fn default_min_count() -> i64 {
    1
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum AudienceStatus {
    Active,
    Archived,
}

/// A versioned audience definition. Registering a new version for the same
/// `(tenant_id, name)` creates a new row with its own id — membership does
/// not carry over automatically, the same way a new schema version doesn't
/// retroactively reinterpret old events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudienceDefinition {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub version: i32,
    pub conditions: Condition,
    pub status: AudienceStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
pub(crate) struct AudienceRow {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub version: i32,
    pub conditions: serde_json::Value,
    pub status: AudienceStatus,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<AudienceRow> for AudienceDefinition {
    type Error = serde_json::Error;

    fn try_from(row: AudienceRow) -> Result<Self, Self::Error> {
        Ok(AudienceDefinition {
            id: row.id,
            tenant_id: row.tenant_id,
            name: row.name,
            version: row.version,
            conditions: serde_json::from_value(row.conditions)?,
            status: row.status,
            created_at: row.created_at,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum MembershipKind {
    Entered,
    Exited,
}

/// Current membership state for one (audience, profile) pair. Re-entering
/// after exiting updates this row in place; full history is in
/// `MembershipEvent` below, which is never overwritten.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Membership {
    pub audience_id: Uuid,
    pub profile_id: Uuid,
    pub entered_at: DateTime<Utc>,
    pub exited_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MembershipEvent {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub audience_id: Uuid,
    pub profile_id: Uuid,
    pub kind: MembershipKind,
    pub created_at: DateTime<Utc>,
}

/// One membership transition made by a single
/// `evaluate_and_sync_membership` call — returned so the caller can log or
/// react to it; the database writes have already happened by the time this
/// is returned.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MembershipChange {
    pub audience_id: Uuid,
    pub kind: MembershipKind,
}
