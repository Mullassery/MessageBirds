use async_trait::async_trait;
use sqlx::PgPool;
use std::collections::BTreeMap;
use thiserror::Error;
use uuid::Uuid;

use mb_schema_registry::FieldDef;

use crate::model::{MixinDef, MixinRow};

pub const STANDARD_NAMESPACE: &str = "core";

#[derive(Debug, Error)]
pub enum MixinError {
    #[error("'{STANDARD_NAMESPACE}' is the reserved namespace for the standard mixin library; custom mixins must use their own namespace")]
    ReservedNamespace,
    #[error("mixin {namespace}/{name}@{version} is already registered")]
    AlreadyExists {
        namespace: String,
        name: String,
        version: String,
    },
    #[error("mixin {namespace}/{name}@{version} is not registered")]
    NotFound {
        namespace: String,
        name: String,
        version: String,
    },
    #[error("no version of mixin '{namespace}/{name}' has been registered")]
    NoVersions { namespace: String, name: String },
    #[error("stored mixin fields failed to deserialize: {0}")]
    Corrupt(#[from] serde_json::Error),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

#[async_trait]
pub trait MixinRepo: Send + Sync {
    /// Registers a tenant-owned custom mixin. Rejected if it tries to use
    /// the reserved `core` namespace.
    async fn register_custom(
        &self,
        namespace: &str,
        name: &str,
        version: &str,
        fields: BTreeMap<String, FieldDef>,
    ) -> Result<MixinDef, MixinError>;

    async fn get(&self, namespace: &str, name: &str, version: &str)
        -> Result<MixinDef, MixinError>;
    async fn get_latest(&self, namespace: &str, name: &str) -> Result<MixinDef, MixinError>;
    async fn list(&self) -> Result<Vec<MixinDef>, MixinError>;
}

pub struct PgMixinRepo {
    pool: PgPool,
}

impl PgMixinRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn insert(
        &self,
        namespace: &str,
        name: &str,
        version: &str,
        fields: &BTreeMap<String, FieldDef>,
    ) -> Result<MixinDef, MixinError> {
        let existing = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM mixins WHERE namespace = $1 AND name = $2 AND version = $3",
        )
        .bind(namespace)
        .bind(name)
        .bind(version)
        .fetch_one(&self.pool)
        .await?;
        if existing > 0 {
            return Err(MixinError::AlreadyExists {
                namespace: namespace.to_string(),
                name: name.to_string(),
                version: version.to_string(),
            });
        }

        let fields_json = serde_json::to_value(fields)?;
        let row = sqlx::query_as::<_, MixinRow>(
            r#"
            INSERT INTO mixins (id, namespace, name, version, fields, status, created_at)
            VALUES ($1, $2, $3, $4, $5, 'active', now())
            RETURNING id, namespace, name, version, fields, status, created_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(namespace)
        .bind(name)
        .bind(version)
        .bind(fields_json)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.try_into()?)
    }

    /// Seeds the standard mixin library into the `core` namespace. Only
    /// called at startup — not exposed through `MixinRepo`, since ordinary
    /// callers must go through `register_custom` and can't touch `core`.
    pub async fn seed_standard_library(&self) -> Result<(), MixinError> {
        for spec in crate::standard::standard_library() {
            match self
                .insert(&spec.namespace, &spec.name, &spec.version, &spec.fields)
                .await
            {
                Ok(_) | Err(MixinError::AlreadyExists { .. }) => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

#[async_trait]
impl MixinRepo for PgMixinRepo {
    async fn register_custom(
        &self,
        namespace: &str,
        name: &str,
        version: &str,
        fields: BTreeMap<String, FieldDef>,
    ) -> Result<MixinDef, MixinError> {
        if namespace == STANDARD_NAMESPACE {
            return Err(MixinError::ReservedNamespace);
        }
        self.insert(namespace, name, version, &fields).await
    }

    async fn get(
        &self,
        namespace: &str,
        name: &str,
        version: &str,
    ) -> Result<MixinDef, MixinError> {
        let row = sqlx::query_as::<_, MixinRow>(
            "SELECT id, namespace, name, version, fields, status, created_at FROM mixins WHERE namespace = $1 AND name = $2 AND version = $3",
        )
        .bind(namespace)
        .bind(name)
        .bind(version)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| MixinError::NotFound {
            namespace: namespace.to_string(),
            name: name.to_string(),
            version: version.to_string(),
        })?;
        Ok(row.try_into()?)
    }

    async fn get_latest(&self, namespace: &str, name: &str) -> Result<MixinDef, MixinError> {
        let row = sqlx::query_as::<_, MixinRow>(
            r#"
            SELECT id, namespace, name, version, fields, status, created_at
            FROM mixins
            WHERE namespace = $1 AND name = $2
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(namespace)
        .bind(name)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| MixinError::NoVersions {
            namespace: namespace.to_string(),
            name: name.to_string(),
        })?;
        Ok(row.try_into()?)
    }

    async fn list(&self) -> Result<Vec<MixinDef>, MixinError> {
        let rows = sqlx::query_as::<_, MixinRow>(
            "SELECT id, namespace, name, version, fields, status, created_at FROM mixins ORDER BY namespace, name, created_at",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(|r| Ok(r.try_into()?)).collect()
    }
}
