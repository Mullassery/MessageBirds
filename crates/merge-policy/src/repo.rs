use async_trait::async_trait;
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use crate::model::{MergePolicy, Strategy};

#[derive(Debug, Error)]
pub enum MergePolicyRepoError {
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

#[async_trait]
pub trait MergePolicyRepo: Send + Sync {
    async fn register(
        &self,
        tenant_id: Uuid,
        name: &str,
        strategy: Strategy,
        config: serde_json::Value,
    ) -> Result<MergePolicy, MergePolicyRepoError>;

    /// The tenant's active policy (most recently registered), or the
    /// implicit default if none has been configured.
    async fn get_active(&self, tenant_id: Uuid) -> Result<MergePolicy, MergePolicyRepoError>;
}

pub struct PgMergePolicyRepo {
    pool: PgPool,
}

impl PgMergePolicyRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MergePolicyRepo for PgMergePolicyRepo {
    async fn register(
        &self,
        tenant_id: Uuid,
        name: &str,
        strategy: Strategy,
        config: serde_json::Value,
    ) -> Result<MergePolicy, MergePolicyRepoError> {
        let next_version: i32 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(version), 0) + 1 FROM merge_policies WHERE tenant_id = $1 AND name = $2",
        )
        .bind(tenant_id)
        .bind(name)
        .fetch_one(&self.pool)
        .await?;

        let row = sqlx::query_as::<_, MergePolicy>(
            r#"
            INSERT INTO merge_policies (id, tenant_id, name, strategy, config, version, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, now())
            RETURNING id, tenant_id, name, strategy, config, version, created_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(tenant_id)
        .bind(name)
        .bind(strategy)
        .bind(config)
        .bind(next_version)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    async fn get_active(&self, tenant_id: Uuid) -> Result<MergePolicy, MergePolicyRepoError> {
        let row = sqlx::query_as::<_, MergePolicy>(
            r#"
            SELECT id, tenant_id, name, strategy, config, version, created_at
            FROM merge_policies
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.unwrap_or_else(|| MergePolicy::default_for_tenant(tenant_id)))
    }
}
