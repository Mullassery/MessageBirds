use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const WEBHOOK_KIND: &str = "webhook";

/// A place data can be activated to. `kind` selects which
/// `DestinationConnector` handles it; `config` is connector-specific (for
/// `webhook`, `{"url": "https://..."}`).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Destination {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub kind: String,
    pub name: String,
    pub config: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ActivationStatus {
    Sent,
    Failed,
}

/// One activation attempt for one profile — the audit record Section 60
/// requires for every activation, even though the governance/consent
/// checks that would normally precede it aren't implemented yet.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ActivationRecord {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub audience_id: Uuid,
    pub destination_id: Uuid,
    pub profile_id: Uuid,
    pub status: ActivationStatus,
    pub detail: Option<String>,
    pub created_at: DateTime<Utc>,
}
