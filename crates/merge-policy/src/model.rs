use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// AEP-inspired merge strategies (Section 10). `SourcePriority` and
/// `LatestTimestamp` are implemented this phase; the rest are named here so
/// the model is forward-compatible, but attempting to apply one returns an
/// explicit `NotImplemented` error rather than silently behaving like a
/// different strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Strategy {
    SourcePriority,
    LatestTimestamp,
    EarliestTimestamp,
    MostTrustedSource,
    ConfidenceWeighted,
    FieldLevel,
    Custom,
}

impl Strategy {
    pub fn is_implemented(self) -> bool {
        matches!(self, Strategy::SourcePriority | Strategy::LatestTimestamp)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MergePolicy {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub strategy: Strategy,
    /// Strategy-specific config, e.g. `{"sources": ["crm", "commerce", "mobile"]}`
    /// for `SourcePriority`.
    pub config: serde_json::Value,
    pub version: i32,
    pub created_at: DateTime<Utc>,
}

impl MergePolicy {
    /// The implicit policy used when a tenant hasn't configured one:
    /// `SourcePriority` with no configured source list, which falls back
    /// to latest-timestamp-wins for every field (see `apply::apply_policy`).
    pub fn default_for_tenant(tenant_id: Uuid) -> Self {
        Self {
            id: Uuid::nil(),
            tenant_id,
            name: "default".to_string(),
            strategy: Strategy::SourcePriority,
            config: serde_json::json!({ "sources": [] }),
            version: 0,
            created_at: Utc::now(),
        }
    }
}

/// One value observed for a profile field, from one source event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldCandidate {
    pub source: String,
    pub value: serde_json::Value,
    pub observed_at: DateTime<Utc>,
}
