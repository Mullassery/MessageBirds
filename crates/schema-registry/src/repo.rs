use async_trait::async_trait;
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use crate::compatibility::check_backward_compatible;
use crate::model::{Schema, SchemaRow};

#[derive(Debug, Error)]
pub enum SchemaError {
    #[error("schema {name}@{version} is already registered")]
    AlreadyExists { name: String, version: String },
    #[error("schema {name}@{version} is not registered")]
    NotFound { name: String, version: String },
    #[error("no version of schema '{0}' has been registered")]
    NoVersions(String),
    #[error("schema {name}@{version} is not backward compatible with {name}@{prev_version}: {reasons:?}")]
    IncompatibleChange {
        name: String,
        version: String,
        prev_version: String,
        reasons: Vec<String>,
    },
    #[error("stored schema fields failed to deserialize: {0}")]
    Corrupt(#[from] serde_json::Error),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

#[async_trait]
pub trait SchemaRepo: Send + Sync {
    async fn register(&self, schema: Schema) -> Result<Schema, SchemaError>;
    async fn get(&self, name: &str, version: &str) -> Result<Schema, SchemaError>;
    async fn get_latest(&self, name: &str) -> Result<Schema, SchemaError>;
    async fn list(&self) -> Result<Vec<Schema>, SchemaError>;
}

pub struct PgSchemaRepo {
    pool: PgPool,
}

impl PgSchemaRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn latest_row(&self, name: &str) -> Result<Option<SchemaRow>, sqlx::Error> {
        sqlx::query_as::<_, SchemaRow>(
            r#"
            SELECT id, name, version, fields, status, created_at
            FROM schemas
            WHERE name = $1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
    }
}

#[async_trait]
impl SchemaRepo for PgSchemaRepo {
    async fn register(&self, schema: Schema) -> Result<Schema, SchemaError> {
        if let Some(prev_row) = self.latest_row(&schema.name).await? {
            let prev: Schema = prev_row.try_into()?;
            if prev.version == schema.version {
                return Err(SchemaError::AlreadyExists {
                    name: schema.name,
                    version: schema.version,
                });
            }
            if let Err(reasons) = check_backward_compatible(&prev, &schema) {
                return Err(SchemaError::IncompatibleChange {
                    name: schema.name.clone(),
                    version: schema.version.clone(),
                    prev_version: prev.version,
                    reasons,
                });
            }
        }

        let fields_json = serde_json::to_value(&schema.fields)?;
        let row = sqlx::query_as::<_, SchemaRow>(
            r#"
            INSERT INTO schemas (id, name, version, fields, status, created_at)
            VALUES ($1, $2, $3, $4, 'active', now())
            RETURNING id, name, version, fields, status, created_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(&schema.name)
        .bind(&schema.version)
        .bind(fields_json)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.try_into()?)
    }

    async fn get(&self, name: &str, version: &str) -> Result<Schema, SchemaError> {
        let row = sqlx::query_as::<_, SchemaRow>(
            "SELECT id, name, version, fields, status, created_at FROM schemas WHERE name = $1 AND version = $2",
        )
        .bind(name)
        .bind(version)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| SchemaError::NotFound {
            name: name.to_string(),
            version: version.to_string(),
        })?;
        Ok(row.try_into()?)
    }

    async fn get_latest(&self, name: &str) -> Result<Schema, SchemaError> {
        let row = self
            .latest_row(name)
            .await?
            .ok_or_else(|| SchemaError::NoVersions(name.to_string()))?;
        Ok(row.try_into()?)
    }

    async fn list(&self) -> Result<Vec<Schema>, SchemaError> {
        let rows = sqlx::query_as::<_, SchemaRow>(
            "SELECT id, name, version, fields, status, created_at FROM schemas ORDER BY name, created_at",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(|r| Ok(r.try_into()?)).collect()
    }
}
