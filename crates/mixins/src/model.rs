use chrono::{DateTime, Utc};
use mb_schema_registry::FieldDef;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
pub enum MixinStatus {
    Active,
    Deprecated,
}

/// A reusable, composable semantic block (`identity`, `person`, `commerce`,
/// or a tenant-owned custom mixin like `acme/fitness_membership`).
/// Namespace ownership keeps custom mixins from colliding with — or
/// silently reshaping — the standard library.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MixinDef {
    pub id: Uuid,
    pub namespace: String,
    pub name: String,
    pub version: String,
    pub fields: BTreeMap<String, FieldDef>,
    pub status: MixinStatus,
    pub created_at: DateTime<Utc>,
}

impl MixinDef {
    pub fn key(&self) -> String {
        format!("{}/{}@{}", self.namespace, self.name, self.version)
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct MixinRow {
    pub id: Uuid,
    pub namespace: String,
    pub name: String,
    pub version: String,
    pub fields: serde_json::Value,
    pub status: MixinStatus,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<MixinRow> for MixinDef {
    type Error = serde_json::Error;

    fn try_from(row: MixinRow) -> Result<Self, Self::Error> {
        Ok(MixinDef {
            id: row.id,
            namespace: row.namespace,
            name: row.name,
            version: row.version,
            fields: serde_json::from_value(row.fields)?,
            status: row.status,
            created_at: row.created_at,
        })
    }
}
