use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const WEBHOOK_KIND: &str = "webhook";

/// A bulk-sync destination, activated through
/// `POST /audiences/{id}/activate-via-reverse-etl` rather than the
/// per-profile `DestinationConnector::send` loop — see
/// `docs/ARCHITECTURE.md`'s Phase 6 section for why PyReverseETL doesn't
/// fit that per-record shape. `config` holds
/// `{"pyreverseetl_workflow_id", "pyreverseetl_activation_id",
/// "pyreverseetl_state_path"?, "pyreverseetl_binary"?}`.
pub const PYREVERSEETL_KIND: &str = "pyreverseetl";

/// A place data can be activated to. `kind` selects which
/// `DestinationConnector` handles it; `config` is connector-specific (for
/// `webhook`, `{"url": "https://..."}`). `supported_actions` is the
/// Section 14 marketing-action vocabulary this destination is declared
/// for (e.g. `["ADVERTISING"]`) — activating for an action not in this
/// list is a `DestinationCapability` denial.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Destination {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub kind: String,
    pub name: String,
    pub config: serde_json::Value,
    pub supported_actions: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ActivationStatus {
    Sent,
    Failed,
    /// Never attempted — a governance policy or missing consent denied it
    /// before send. Distinct from `Failed` (attempted, connector errored).
    Blocked,
}

/// One activation attempt for one profile — the audit record Section 60
/// requires for every activation. `status` reflects both delivery
/// failures and governance/consent denials (`detail` carries which).
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

/// One row of `pyreverseetl_export_staging` — the flattened, standard-mixin
/// subset of a profile that PyReverseETL's Postgres source connector reads.
#[derive(Debug, Clone)]
pub struct ExportRow {
    pub profile_id: Uuid,
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub phone: Option<String>,
}
