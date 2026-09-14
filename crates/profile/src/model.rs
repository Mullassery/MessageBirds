use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The unified customer profile: a projection built from event streams via
/// mixin composition and a merge policy, not a hand-maintained record.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Profile {
    pub id: Uuid,
    pub tenant_id: Uuid,
    /// `{"core/person@1.0": {"first_name": "Jane"}, "core/contact@1.0": {...}}`
    pub mixins: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Which source won a field, and from which event — the seed for full
/// lineage (Section 12). Only the winning value's provenance is kept in
/// this phase; a full "every observation" ledger is a later enhancement.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct FieldProvenance {
    pub profile_id: Uuid,
    pub mixin_key: String,
    pub field_path: String,
    pub value: serde_json::Value,
    pub source: String,
    pub event_id: Uuid,
    pub applied_policy: String,
    pub updated_at: DateTime<Utc>,
}
