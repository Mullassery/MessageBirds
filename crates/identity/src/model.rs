use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct IdentityNode {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub namespace: String,
    pub value_hash: String,
    pub profile_id: Uuid,
    pub confidence: f64,
    pub source: String,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum IdentityAuditKind {
    Linked,
    Merged,
    Split,
}

/// Append-only record of every identity graph change. Never updated or
/// deleted — identity history must stay reconstructable (Section 8).
///
/// `namespace`/`value_hash` are `None` for `Merged` entries, which record a
/// whole-profile merge rather than one identity claim.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct IdentityAuditEntry {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub kind: IdentityAuditKind,
    pub namespace: Option<String>,
    pub value_hash: Option<String>,
    pub profile_id: Uuid,
    pub previous_profile_id: Option<Uuid>,
    pub source: String,
    pub created_at: DateTime<Utc>,
}

/// Result of resolving a set of identity claims against the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedIdentity {
    pub profile_id: Uuid,
    /// Other profile ids that turned out to be the same customer and were
    /// merged into `profile_id` as part of this resolution.
    pub merged_profile_ids: Vec<Uuid>,
}
