use async_trait::async_trait;
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use crate::model::MessageTemplate;

#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("template {0} is not registered")]
    NotFound(Uuid),
    #[error("no version of template '{0}' has been registered")]
    NoVersions(String),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

#[async_trait]
pub trait TemplateRepo: Send + Sync {
    async fn register(
        &self,
        tenant_id: Uuid,
        channel_kind: &str,
        name: &str,
        subject: Option<String>,
        body: String,
    ) -> Result<MessageTemplate, TemplateError>;

    /// Look up by id — what `mb-journeys` Action nodes pin to, since a
    /// journey should keep using the exact template version it was
    /// authored against.
    async fn get(&self, tenant_id: Uuid, id: Uuid) -> Result<MessageTemplate, TemplateError>;

    async fn get_latest(
        &self,
        tenant_id: Uuid,
        name: &str,
    ) -> Result<MessageTemplate, TemplateError>;

    async fn list(&self, tenant_id: Uuid) -> Result<Vec<MessageTemplate>, TemplateError>;
}

pub struct PgTemplateRepo {
    pool: PgPool,
}

impl PgTemplateRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const TEMPLATE_COLUMNS: &str =
    "id, tenant_id, channel_kind, name, version, subject, body, created_at";

#[async_trait]
impl TemplateRepo for PgTemplateRepo {
    async fn register(
        &self,
        tenant_id: Uuid,
        channel_kind: &str,
        name: &str,
        subject: Option<String>,
        body: String,
    ) -> Result<MessageTemplate, TemplateError> {
        let next_version: i32 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(version), 0) + 1 FROM message_templates WHERE tenant_id = $1 AND name = $2",
        )
        .bind(tenant_id)
        .bind(name)
        .fetch_one(&self.pool)
        .await?;

        let row = sqlx::query_as::<_, MessageTemplate>(&format!(
            r#"
            INSERT INTO message_templates (id, tenant_id, channel_kind, name, version, subject, body, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, now())
            RETURNING {TEMPLATE_COLUMNS}
            "#
        ))
        .bind(Uuid::new_v4())
        .bind(tenant_id)
        .bind(channel_kind)
        .bind(name)
        .bind(next_version)
        .bind(subject)
        .bind(body)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    async fn get(&self, tenant_id: Uuid, id: Uuid) -> Result<MessageTemplate, TemplateError> {
        sqlx::query_as::<_, MessageTemplate>(&format!(
            "SELECT {TEMPLATE_COLUMNS} FROM message_templates WHERE tenant_id = $1 AND id = $2"
        ))
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(TemplateError::NotFound(id))
    }

    async fn get_latest(
        &self,
        tenant_id: Uuid,
        name: &str,
    ) -> Result<MessageTemplate, TemplateError> {
        sqlx::query_as::<_, MessageTemplate>(&format!(
            r#"
            SELECT {TEMPLATE_COLUMNS} FROM message_templates
            WHERE tenant_id = $1 AND name = $2
            ORDER BY version DESC
            LIMIT 1
            "#
        ))
        .bind(tenant_id)
        .bind(name)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| TemplateError::NoVersions(name.to_string()))
    }

    async fn list(&self, tenant_id: Uuid) -> Result<Vec<MessageTemplate>, TemplateError> {
        let rows = sqlx::query_as::<_, MessageTemplate>(&format!(
            "SELECT {TEMPLATE_COLUMNS} FROM message_templates WHERE tenant_id = $1 ORDER BY name, version"
        ))
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}
