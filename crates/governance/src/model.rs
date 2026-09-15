use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Effect {
    Allow,
    Deny,
}

/// `IF label == X AND action == Y THEN effect`. A deny-list, not a full
/// rules engine — no exceptions/inheritance/versioning (Section 15
/// mentions these; not implemented). Default is allow; only a matching
/// `Deny` policy blocks anything.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Policy {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub label: String,
    pub action: String,
    pub effect: Effect,
    pub priority: i32,
    pub created_at: DateTime<Utc>,
}

/// Append-only. Never updated or deleted — consent history must stay
/// reconstructable, same principle as `identity_audit` /
/// `audience_membership_events`.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ConsentEvent {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub profile_id: Uuid,
    pub purpose: String,
    pub granted: bool,
    pub source: String,
    pub jurisdiction: Option<String>,
    pub consent_version: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Derived: the latest non-expired consent event for one purpose.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentState {
    pub purpose: String,
    pub granted: bool,
    pub source: String,
    pub updated_at: DateTime<Utc>,
}

/// Why an activation was or wasn't allowed — structured, not a string, so
/// "why was this blocked?" (Section 16) is always fully answerable, never
/// just "policy violation."
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Reason {
    LabelPolicyDenied {
        label: String,
        action: String,
        policy_id: Uuid,
    },
    ConsentMissing {
        purpose: String,
    },
    DestinationCapability {
        action: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub allowed: bool,
    pub reasons: Vec<Reason>,
}
