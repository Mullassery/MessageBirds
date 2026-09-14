use async_trait::async_trait;
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use crate::model::{Namespace, NamespaceKind};

#[derive(Debug, Error)]
pub enum NamespaceError {
    #[error("namespace '{0}' is already registered")]
    AlreadyExists(String),
    #[error("namespace '{0}' is not registered")]
    NotFound(String),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

#[async_trait]
pub trait NamespaceRepo: Send + Sync {
    async fn register(
        &self,
        key: &str,
        kind: NamespaceKind,
        priority: i32,
    ) -> Result<Namespace, NamespaceError>;

    async fn get(&self, key: &str) -> Result<Namespace, NamespaceError>;

    async fn list(&self) -> Result<Vec<Namespace>, NamespaceError>;
}

pub struct PgNamespaceRepo {
    pool: PgPool,
}

impl PgNamespaceRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl NamespaceRepo for PgNamespaceRepo {
    async fn register(
        &self,
        key: &str,
        kind: NamespaceKind,
        priority: i32,
    ) -> Result<Namespace, NamespaceError> {
        let existing =
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM namespaces WHERE key = $1")
                .bind(key)
                .fetch_one(&self.pool)
                .await?;
        if existing > 0 {
            return Err(NamespaceError::AlreadyExists(key.to_string()));
        }

        let kind_str = match kind {
            NamespaceKind::Deterministic => "deterministic",
            NamespaceKind::Probabilistic => "probabilistic",
        };

        let row = sqlx::query_as::<_, Namespace>(
            r#"
            INSERT INTO namespaces (id, key, kind, priority, created_at)
            VALUES ($1, $2, $3, $4, now())
            RETURNING id, key, kind, priority, created_at
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(key)
        .bind(kind_str)
        .bind(priority)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    async fn get(&self, key: &str) -> Result<Namespace, NamespaceError> {
        sqlx::query_as::<_, Namespace>(
            "SELECT id, key, kind, priority, created_at FROM namespaces WHERE key = $1",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| NamespaceError::NotFound(key.to_string()))
    }

    async fn list(&self) -> Result<Vec<Namespace>, NamespaceError> {
        let rows = sqlx::query_as::<_, Namespace>(
            "SELECT id, key, kind, priority, created_at FROM namespaces ORDER BY priority DESC, key",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}
