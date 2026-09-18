use async_trait::async_trait;
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use crate::model::{ActivationRecord, ActivationStatus, Destination, ExportRow};

#[derive(Debug, Error)]
pub enum ConnectorRepoError {
    #[error("destination {0} is not registered")]
    NotFound(Uuid),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

#[async_trait]
pub trait DestinationRepo: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    async fn register(
        &self,
        tenant_id: Uuid,
        kind: &str,
        name: &str,
        config: serde_json::Value,
        supported_actions: Vec<String>,
    ) -> Result<Destination, ConnectorRepoError>;

    async fn get(&self, tenant_id: Uuid, id: Uuid) -> Result<Destination, ConnectorRepoError>;

    async fn list(&self, tenant_id: Uuid) -> Result<Vec<Destination>, ConnectorRepoError>;
}

#[async_trait]
pub trait ActivationRepo: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    async fn log(
        &self,
        tenant_id: Uuid,
        audience_id: Uuid,
        destination_id: Uuid,
        profile_id: Uuid,
        status: ActivationStatus,
        detail: Option<String>,
    ) -> Result<(), ConnectorRepoError>;

    async fn list_for_audience(
        &self,
        audience_id: Uuid,
    ) -> Result<Vec<ActivationRecord>, ConnectorRepoError>;
}

/// Backs the `pyreverseetl` destination kind's bulk activation path —
/// separate from `DestinationRepo`/`ActivationRepo` since it's a
/// different concern (staging rows for an external sync tool to read),
/// but implemented by the same `PgConnectorRepo`, same rationale as the
/// other two traits it already carries.
#[async_trait]
pub trait PyReverseEtlExportRepo: Send + Sync {
    /// Replaces every staged row for `destination_id` with `rows` in one
    /// transaction — each run only ever sees the members that just passed
    /// the consent check, not stale rows from a previous run.
    async fn replace_staging(
        &self,
        tenant_id: Uuid,
        destination_id: Uuid,
        rows: Vec<ExportRow>,
    ) -> Result<(), ConnectorRepoError>;
}

pub struct PgConnectorRepo {
    pool: PgPool,
}

impl PgConnectorRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DestinationRepo for PgConnectorRepo {
    async fn register(
        &self,
        tenant_id: Uuid,
        kind: &str,
        name: &str,
        config: serde_json::Value,
        supported_actions: Vec<String>,
    ) -> Result<Destination, ConnectorRepoError> {
        let row = sqlx::query_as::<_, Destination>(
            r#"
            INSERT INTO destinations (id, tenant_id, kind, name, config, supported_actions, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, now())
            RETURNING id, tenant_id, kind, name, config, supported_actions, created_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(tenant_id)
        .bind(kind)
        .bind(name)
        .bind(config)
        .bind(supported_actions)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get(&self, tenant_id: Uuid, id: Uuid) -> Result<Destination, ConnectorRepoError> {
        sqlx::query_as::<_, Destination>(
            "SELECT id, tenant_id, kind, name, config, supported_actions, created_at FROM destinations WHERE tenant_id = $1 AND id = $2",
        )
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(ConnectorRepoError::NotFound(id))
    }

    async fn list(&self, tenant_id: Uuid) -> Result<Vec<Destination>, ConnectorRepoError> {
        let rows = sqlx::query_as::<_, Destination>(
            "SELECT id, tenant_id, kind, name, config, supported_actions, created_at FROM destinations WHERE tenant_id = $1 ORDER BY name",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}

#[async_trait]
impl ActivationRepo for PgConnectorRepo {
    async fn log(
        &self,
        tenant_id: Uuid,
        audience_id: Uuid,
        destination_id: Uuid,
        profile_id: Uuid,
        status: ActivationStatus,
        detail: Option<String>,
    ) -> Result<(), ConnectorRepoError> {
        sqlx::query(
            r#"
            INSERT INTO activation_log (id, tenant_id, audience_id, destination_id, profile_id, status, detail, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, now())
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(tenant_id)
        .bind(audience_id)
        .bind(destination_id)
        .bind(profile_id)
        .bind(status)
        .bind(detail)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list_for_audience(
        &self,
        audience_id: Uuid,
    ) -> Result<Vec<ActivationRecord>, ConnectorRepoError> {
        let rows = sqlx::query_as::<_, ActivationRecord>(
            r#"
            SELECT id, tenant_id, audience_id, destination_id, profile_id, status, detail, created_at
            FROM activation_log
            WHERE audience_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(audience_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}

#[async_trait]
impl PyReverseEtlExportRepo for PgConnectorRepo {
    async fn replace_staging(
        &self,
        tenant_id: Uuid,
        destination_id: Uuid,
        rows: Vec<ExportRow>,
    ) -> Result<(), ConnectorRepoError> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM pyreverseetl_export_staging WHERE destination_id = $1")
            .bind(destination_id)
            .execute(&mut *tx)
            .await?;

        for row in rows {
            sqlx::query(
                r#"
                INSERT INTO pyreverseetl_export_staging
                    (id, tenant_id, destination_id, profile_id, email, first_name, last_name, phone, created_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, now())
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(tenant_id)
            .bind(destination_id)
            .bind(row.profile_id)
            .bind(row.email)
            .bind(row.first_name)
            .bind(row.last_name)
            .bind(row.phone)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }
}
