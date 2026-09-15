use async_recursion::async_recursion;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use crate::eval::{eval_attribute_op, get_field};
use crate::model::{
    AudienceDefinition, AudienceRow, Condition, Membership, MembershipChange, MembershipKind,
};

#[derive(Debug, Error)]
pub enum AudienceError {
    #[error("audience {0} is not registered")]
    NotFound(Uuid),
    #[error("stored audience conditions failed to deserialize: {0}")]
    Corrupt(#[from] serde_json::Error),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

#[async_trait]
pub trait AudienceRepo: Send + Sync {
    async fn register(
        &self,
        tenant_id: Uuid,
        name: &str,
        conditions: Condition,
    ) -> Result<AudienceDefinition, AudienceError>;

    async fn get(&self, tenant_id: Uuid, id: Uuid) -> Result<AudienceDefinition, AudienceError>;

    async fn list(&self, tenant_id: Uuid) -> Result<Vec<AudienceDefinition>, AudienceError>;

    async fn get_members(&self, audience_id: Uuid) -> Result<Vec<Membership>, AudienceError>;

    /// Every membership row (current + historical) touching this profile,
    /// across all of the tenant's audiences.
    async fn get_profile_audiences(
        &self,
        tenant_id: Uuid,
        profile_id: Uuid,
    ) -> Result<Vec<Membership>, AudienceError>;

    /// Re-evaluates every active audience for `tenant_id` against a
    /// profile's current mixins and syncs `audience_memberships` /
    /// `audience_membership_events` to match. Only reacts to the profile
    /// whose event just arrived — an audience whose condition becomes
    /// false for reasons unrelated to that profile receiving an event
    /// (e.g. a pure time-window expiring) won't exit that profile until
    /// it's re-evaluated by another of its own events.
    async fn evaluate_and_sync_membership(
        &self,
        tenant_id: Uuid,
        profile_id: Uuid,
        mixins: &Value,
    ) -> Result<Vec<MembershipChange>, AudienceError>;
}

pub struct PgAudienceRepo {
    pool: PgPool,
}

impl PgAudienceRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    #[async_recursion]
    async fn matches(
        &self,
        condition: &Condition,
        mixins: &Value,
        tenant_id: Uuid,
        profile_id: Uuid,
    ) -> Result<bool, AudienceError> {
        match condition {
            Condition::Attribute {
                mixin,
                field,
                op,
                value,
            } => {
                let actual = get_field(mixins, mixin, field);
                Ok(eval_attribute_op(*op, actual, value.as_ref()))
            }
            Condition::Event {
                event_type,
                within_days,
                min_count,
            } => {
                let count: i64 = sqlx::query_scalar(
                    r#"
                    SELECT count(*) FROM events
                    WHERE tenant_id = $1 AND profile_id = $2 AND event_type = $3
                      AND occurred_at >= now() - ($4 || ' days')::interval
                    "#,
                )
                .bind(tenant_id)
                .bind(profile_id)
                .bind(event_type)
                .bind(within_days.to_string())
                .fetch_one(&self.pool)
                .await?;
                Ok(count >= *min_count)
            }
            Condition::And(conditions) => {
                for c in conditions {
                    if !self.matches(c, mixins, tenant_id, profile_id).await? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            Condition::Or(conditions) => {
                for c in conditions {
                    if self.matches(c, mixins, tenant_id, profile_id).await? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            Condition::Not(inner) => {
                Ok(!self.matches(inner, mixins, tenant_id, profile_id).await?)
            }
        }
    }
}

const AUDIENCE_COLUMNS: &str = "id, tenant_id, name, version, conditions, status, created_at";

#[async_trait]
impl AudienceRepo for PgAudienceRepo {
    async fn register(
        &self,
        tenant_id: Uuid,
        name: &str,
        conditions: Condition,
    ) -> Result<AudienceDefinition, AudienceError> {
        let next_version: i32 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(version), 0) + 1 FROM audiences WHERE tenant_id = $1 AND name = $2",
        )
        .bind(tenant_id)
        .bind(name)
        .fetch_one(&self.pool)
        .await?;

        let conditions_json = serde_json::to_value(&conditions)?;

        let row = sqlx::query_as::<_, AudienceRow>(&format!(
            r#"
            INSERT INTO audiences (id, tenant_id, name, version, conditions, status, created_at)
            VALUES ($1, $2, $3, $4, $5, 'active', now())
            RETURNING {AUDIENCE_COLUMNS}
            "#
        ))
        .bind(Uuid::new_v4())
        .bind(tenant_id)
        .bind(name)
        .bind(next_version)
        .bind(conditions_json)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.try_into()?)
    }

    async fn get(&self, tenant_id: Uuid, id: Uuid) -> Result<AudienceDefinition, AudienceError> {
        let row = sqlx::query_as::<_, AudienceRow>(&format!(
            "SELECT {AUDIENCE_COLUMNS} FROM audiences WHERE tenant_id = $1 AND id = $2"
        ))
        .bind(tenant_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AudienceError::NotFound(id))?;
        Ok(row.try_into()?)
    }

    async fn list(&self, tenant_id: Uuid) -> Result<Vec<AudienceDefinition>, AudienceError> {
        let rows = sqlx::query_as::<_, AudienceRow>(&format!(
            "SELECT {AUDIENCE_COLUMNS} FROM audiences WHERE tenant_id = $1 ORDER BY name, created_at"
        ))
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(|r| Ok(r.try_into()?)).collect()
    }

    async fn get_members(&self, audience_id: Uuid) -> Result<Vec<Membership>, AudienceError> {
        let rows = sqlx::query_as::<_, Membership>(
            r#"
            SELECT audience_id, profile_id, entered_at, exited_at
            FROM audience_memberships
            WHERE audience_id = $1 AND exited_at IS NULL
            ORDER BY entered_at
            "#,
        )
        .bind(audience_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn get_profile_audiences(
        &self,
        tenant_id: Uuid,
        profile_id: Uuid,
    ) -> Result<Vec<Membership>, AudienceError> {
        let rows = sqlx::query_as::<_, Membership>(
            r#"
            SELECT m.audience_id, m.profile_id, m.entered_at, m.exited_at
            FROM audience_memberships m
            JOIN audiences a ON a.id = m.audience_id
            WHERE a.tenant_id = $1 AND m.profile_id = $2
            ORDER BY m.entered_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(profile_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn evaluate_and_sync_membership(
        &self,
        tenant_id: Uuid,
        profile_id: Uuid,
        mixins: &Value,
    ) -> Result<Vec<MembershipChange>, AudienceError> {
        let active = sqlx::query_as::<_, AudienceRow>(&format!(
            "SELECT {AUDIENCE_COLUMNS} FROM audiences WHERE tenant_id = $1 AND status = 'active'"
        ))
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        let mut changes = Vec::new();

        for row in active {
            let audience: AudienceDefinition = row.try_into()?;
            let is_member_now = self
                .matches(&audience.conditions, mixins, tenant_id, profile_id)
                .await?;

            let existing: Option<Option<DateTime<Utc>>> = sqlx::query_scalar(
                "SELECT exited_at FROM audience_memberships WHERE audience_id = $1 AND profile_id = $2",
            )
            .bind(audience.id)
            .bind(profile_id)
            .fetch_optional(&self.pool)
            .await?;

            let currently_active = matches!(existing, Some(None));

            if is_member_now && !currently_active {
                sqlx::query(
                    r#"
                    INSERT INTO audience_memberships (audience_id, profile_id, entered_at, exited_at)
                    VALUES ($1, $2, now(), NULL)
                    ON CONFLICT (audience_id, profile_id) DO UPDATE SET entered_at = now(), exited_at = NULL
                    "#,
                )
                .bind(audience.id)
                .bind(profile_id)
                .execute(&self.pool)
                .await?;

                self.log_membership_event(tenant_id, audience.id, profile_id, "entered")
                    .await?;
                changes.push(MembershipChange {
                    audience_id: audience.id,
                    kind: MembershipKind::Entered,
                });
            } else if !is_member_now && currently_active {
                sqlx::query(
                    "UPDATE audience_memberships SET exited_at = now() WHERE audience_id = $1 AND profile_id = $2",
                )
                .bind(audience.id)
                .bind(profile_id)
                .execute(&self.pool)
                .await?;

                self.log_membership_event(tenant_id, audience.id, profile_id, "exited")
                    .await?;
                changes.push(MembershipChange {
                    audience_id: audience.id,
                    kind: MembershipKind::Exited,
                });
            }
        }

        Ok(changes)
    }
}

impl PgAudienceRepo {
    async fn log_membership_event(
        &self,
        tenant_id: Uuid,
        audience_id: Uuid,
        profile_id: Uuid,
        kind: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO audience_membership_events (id, tenant_id, audience_id, profile_id, kind, created_at)
            VALUES ($1, $2, $3, $4, $5, now())
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(tenant_id)
        .bind(audience_id)
        .bind(profile_id)
        .bind(kind)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
