use async_trait::async_trait;
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use crate::model::Channel;

#[derive(Debug, Error)]
pub enum ChannelRepoError {
    #[error("channel {0} is not registered")]
    NotFound(Uuid),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

#[async_trait]
pub trait ChannelRepo: Send + Sync {
    async fn register(
        &self,
        tenant_id: Uuid,
        kind: &str,
        name: &str,
        config: serde_json::Value,
    ) -> Result<Channel, ChannelRepoError>;

    async fn get(&self, tenant_id: Uuid, id: Uuid) -> Result<Channel, ChannelRepoError>;

    async fn list(&self, tenant_id: Uuid) -> Result<Vec<Channel>, ChannelRepoError>;
}

pub struct PgChannelRepo {
    pool: PgPool,
}

impl PgChannelRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const CHANNEL_COLUMNS: &str = "id, tenant_id, kind, name, config, created_at";

#[async_trait]
impl ChannelRepo for PgChannelRepo {
    async fn register(
        &self,
        tenant_id: Uuid,
        kind: &str,
        name: &str,
        config: serde_json::Value,
    ) -> Result<Channel, ChannelRepoError> {
        let row = sqlx::query_as::<_, Channel>(&format!(
            r#"
            INSERT INTO channels (id, tenant_id, kind, name, config, created_at)
            VALUES ($1, $2, $3, $4, $5, now())
            RETURNING {CHANNEL_COLUMNS}
            "#
        ))
        .bind(Uuid::new_v4())
        .bind(tenant_id)
        .bind(kind)
        .bind(name)
        .bind(config)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get(&self, tenant_id: Uuid, id: Uuid) -> Result<Channel, ChannelRepoError> {
        sqlx::query_as::<_, Channel>(&format!(
            "SELECT {CHANNEL_COLUMNS} FROM channels WHERE tenant_id = $1 AND id = $2"
        ))
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(ChannelRepoError::NotFound(id))
    }

    async fn list(&self, tenant_id: Uuid) -> Result<Vec<Channel>, ChannelRepoError> {
        let rows = sqlx::query_as::<_, Channel>(&format!(
            "SELECT {CHANNEL_COLUMNS} FROM channels WHERE tenant_id = $1 ORDER BY name"
        ))
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}
