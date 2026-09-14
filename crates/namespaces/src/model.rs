use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
pub enum NamespaceKind {
    Deterministic,
    Probabilistic,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Namespace {
    pub id: Uuid,
    pub key: String,
    pub kind: NamespaceKind,
    /// Higher priority namespaces are preferred when resolving identity
    /// conflicts (e.g. `customer_id` should usually outrank `anonymous_id`).
    pub priority: i32,
    pub created_at: DateTime<Utc>,
}
