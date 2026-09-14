use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    String,
    Number,
    Boolean,
    Object,
    Array,
}

impl FieldType {
    pub fn matches(&self, value: &serde_json::Value) -> bool {
        match self {
            FieldType::String => value.is_string(),
            FieldType::Number => value.is_number(),
            FieldType::Boolean => value.is_boolean(),
            FieldType::Object => value.is_object(),
            FieldType::Array => value.is_array(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDef {
    #[serde(rename = "type")]
    pub field_type: FieldType,
    #[serde(default)]
    pub required: bool,
    #[serde(default, rename = "enum")]
    pub enum_values: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
pub enum SchemaStatus {
    Active,
    Deprecated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub fields: BTreeMap<String, FieldDef>,
    pub status: SchemaStatus,
    pub created_at: DateTime<Utc>,
}

/// Row shape as stored: `fields` is JSONB, decoded into `BTreeMap<String, FieldDef>`
/// by the repo layer rather than derived directly, since sqlx's `FromRow`
/// can't decode a JSONB column straight into a non-`Json<T>` field.
#[derive(sqlx::FromRow)]
pub(crate) struct SchemaRow {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub fields: serde_json::Value,
    pub status: SchemaStatus,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<SchemaRow> for Schema {
    type Error = serde_json::Error;

    fn try_from(row: SchemaRow) -> Result<Self, Self::Error> {
        Ok(Schema {
            id: row.id,
            name: row.name,
            version: row.version,
            fields: serde_json::from_value(row.fields)?,
            status: row.status,
            created_at: row.created_at,
        })
    }
}
