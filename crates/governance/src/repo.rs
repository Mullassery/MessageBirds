use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use crate::model::{ConsentEvent, ConsentState, Effect, Policy, Reason};

#[derive(Debug, Error)]
pub enum GovernanceError {
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

#[async_trait]
pub trait PolicyRepo: Send + Sync {
    async fn register(
        &self,
        tenant_id: Uuid,
        label: &str,
        action: &str,
        effect: Effect,
        priority: i32,
    ) -> Result<Policy, GovernanceError>;

    async fn list(&self, tenant_id: Uuid) -> Result<Vec<Policy>, GovernanceError>;

    /// Every `Deny` policy matching any of `labels` for `action`, as
    /// explainable reasons. Empty means nothing blocks this — the engine
    /// is default-allow; only an explicit `Deny` policy stops anything.
    async fn evaluate_labels(
        &self,
        tenant_id: Uuid,
        labels: &[String],
        action: &str,
    ) -> Result<Vec<Reason>, GovernanceError>;
}

#[async_trait]
pub trait ConsentRepo: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    async fn record(
        &self,
        tenant_id: Uuid,
        profile_id: Uuid,
        purpose: &str,
        granted: bool,
        source: &str,
        jurisdiction: Option<String>,
        consent_version: Option<String>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<ConsentEvent, GovernanceError>;

    async fn get_current(
        &self,
        tenant_id: Uuid,
        profile_id: Uuid,
        purpose: &str,
    ) -> Result<Option<ConsentState>, GovernanceError>;

    async fn get_all_current(
        &self,
        tenant_id: Uuid,
        profile_id: Uuid,
    ) -> Result<Vec<ConsentState>, GovernanceError>;

    async fn get_history(
        &self,
        tenant_id: Uuid,
        profile_id: Uuid,
    ) -> Result<Vec<ConsentEvent>, GovernanceError>;
}

pub struct PgGovernanceRepo {
    pool: PgPool,
}

impl PgGovernanceRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const POLICY_COLUMNS: &str = "id, tenant_id, label, action, effect, priority, created_at";
const CONSENT_COLUMNS: &str =
    "id, tenant_id, profile_id, purpose, granted, source, jurisdiction, consent_version, expires_at, created_at";

fn to_state(e: ConsentEvent) -> ConsentState {
    ConsentState {
        purpose: e.purpose,
        granted: e.granted,
        source: e.source,
        updated_at: e.created_at,
    }
}

#[async_trait]
impl PolicyRepo for PgGovernanceRepo {
    async fn register(
        &self,
        tenant_id: Uuid,
        label: &str,
        action: &str,
        effect: Effect,
        priority: i32,
    ) -> Result<Policy, GovernanceError> {
        let row = sqlx::query_as::<_, Policy>(&format!(
            r#"
            INSERT INTO policies (id, tenant_id, label, action, effect, priority, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, now())
            RETURNING {POLICY_COLUMNS}
            "#
        ))
        .bind(Uuid::new_v4())
        .bind(tenant_id)
        .bind(label)
        .bind(action)
        .bind(effect)
        .bind(priority)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list(&self, tenant_id: Uuid) -> Result<Vec<Policy>, GovernanceError> {
        let rows = sqlx::query_as::<_, Policy>(&format!(
            "SELECT {POLICY_COLUMNS} FROM policies WHERE tenant_id = $1 ORDER BY priority DESC, created_at"
        ))
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn evaluate_labels(
        &self,
        tenant_id: Uuid,
        labels: &[String],
        action: &str,
    ) -> Result<Vec<Reason>, GovernanceError> {
        if labels.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query_as::<_, Policy>(&format!(
            r#"
            SELECT {POLICY_COLUMNS} FROM policies
            WHERE tenant_id = $1 AND action = $2 AND label = ANY($3) AND effect = 'deny'
            ORDER BY priority DESC
            "#
        ))
        .bind(tenant_id)
        .bind(action)
        .bind(labels)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|p| Reason::LabelPolicyDenied {
                label: p.label,
                action: p.action,
                policy_id: p.id,
            })
            .collect())
    }
}

#[async_trait]
impl ConsentRepo for PgGovernanceRepo {
    async fn record(
        &self,
        tenant_id: Uuid,
        profile_id: Uuid,
        purpose: &str,
        granted: bool,
        source: &str,
        jurisdiction: Option<String>,
        consent_version: Option<String>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<ConsentEvent, GovernanceError> {
        let row = sqlx::query_as::<_, ConsentEvent>(&format!(
            r#"
            INSERT INTO consent_events (id, tenant_id, profile_id, purpose, granted, source, jurisdiction, consent_version, expires_at, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, now())
            RETURNING {CONSENT_COLUMNS}
            "#
        ))
        .bind(Uuid::new_v4())
        .bind(tenant_id)
        .bind(profile_id)
        .bind(purpose)
        .bind(granted)
        .bind(source)
        .bind(jurisdiction)
        .bind(consent_version)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_current(
        &self,
        tenant_id: Uuid,
        profile_id: Uuid,
        purpose: &str,
    ) -> Result<Option<ConsentState>, GovernanceError> {
        let row = sqlx::query_as::<_, ConsentEvent>(&format!(
            r#"
            SELECT {CONSENT_COLUMNS} FROM consent_events
            WHERE tenant_id = $1 AND profile_id = $2 AND purpose = $3
              AND (expires_at IS NULL OR expires_at > now())
            ORDER BY created_at DESC
            LIMIT 1
            "#
        ))
        .bind(tenant_id)
        .bind(profile_id)
        .bind(purpose)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(to_state))
    }

    async fn get_all_current(
        &self,
        tenant_id: Uuid,
        profile_id: Uuid,
    ) -> Result<Vec<ConsentState>, GovernanceError> {
        let rows = sqlx::query_as::<_, ConsentEvent>(&format!(
            r#"
            SELECT DISTINCT ON (purpose) {CONSENT_COLUMNS} FROM consent_events
            WHERE tenant_id = $1 AND profile_id = $2
              AND (expires_at IS NULL OR expires_at > now())
            ORDER BY purpose, created_at DESC
            "#
        ))
        .bind(tenant_id)
        .bind(profile_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(to_state).collect())
    }

    async fn get_history(
        &self,
        tenant_id: Uuid,
        profile_id: Uuid,
    ) -> Result<Vec<ConsentEvent>, GovernanceError> {
        let rows = sqlx::query_as::<_, ConsentEvent>(&format!(
            r#"
            SELECT {CONSENT_COLUMNS} FROM consent_events
            WHERE tenant_id = $1 AND profile_id = $2
            ORDER BY created_at DESC
            "#
        ))
        .bind(tenant_id)
        .bind(profile_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}
