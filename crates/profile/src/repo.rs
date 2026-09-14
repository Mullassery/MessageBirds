use async_trait::async_trait;
use chrono::Utc;
use mb_merge_policy::{FieldCandidate, MergePolicy};
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use crate::decide::{decide, Decision};
use crate::model::{FieldProvenance, Profile};

#[derive(Debug, Error)]
pub enum ProfileError {
    #[error("mixin update payload must be a JSON object")]
    NotAnObject,
    #[error(transparent)]
    MergePolicy(#[from] mb_merge_policy::MergePolicyError),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

#[async_trait]
pub trait ProfileRepo: Send + Sync {
    async fn get(&self, profile_id: Uuid) -> Result<Option<Profile>, ProfileError>;

    async fn get_provenance(&self, profile_id: Uuid) -> Result<Vec<FieldProvenance>, ProfileError>;

    /// Applies one mixin's worth of field values from one event onto a
    /// profile, running each field through the merge policy against
    /// whatever value currently holds provenance. `data` must already be
    /// validated against the mixin's schema by the caller.
    #[allow(clippy::too_many_arguments)]
    async fn apply_mixin_update(
        &self,
        tenant_id: Uuid,
        profile_id: Uuid,
        mixin_key: &str,
        data: &serde_json::Value,
        source: &str,
        event_id: Uuid,
        policy: &MergePolicy,
    ) -> Result<(), ProfileError>;
}

pub struct PgProfileRepo {
    pool: PgPool,
}

impl PgProfileRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProfileRepo for PgProfileRepo {
    async fn get(&self, profile_id: Uuid) -> Result<Option<Profile>, ProfileError> {
        let row = sqlx::query_as::<_, Profile>(
            "SELECT id, tenant_id, mixins, created_at, updated_at FROM profiles WHERE id = $1",
        )
        .bind(profile_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_provenance(&self, profile_id: Uuid) -> Result<Vec<FieldProvenance>, ProfileError> {
        let rows = sqlx::query_as::<_, FieldProvenance>(
            r#"
            SELECT profile_id, mixin_key, field_path, value, source, event_id, applied_policy, updated_at
            FROM profile_field_provenance
            WHERE profile_id = $1
            ORDER BY mixin_key, field_path
            "#,
        )
        .bind(profile_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    #[allow(clippy::too_many_arguments)]
    async fn apply_mixin_update(
        &self,
        tenant_id: Uuid,
        profile_id: Uuid,
        mixin_key: &str,
        data: &serde_json::Value,
        source: &str,
        event_id: Uuid,
        policy: &MergePolicy,
    ) -> Result<(), ProfileError> {
        let fields = data.as_object().ok_or(ProfileError::NotAnObject)?;
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO profiles (id, tenant_id, mixins, created_at, updated_at)
            VALUES ($1, $2, '{}'::jsonb, now(), now())
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(profile_id)
        .bind(tenant_id)
        .execute(&mut *tx)
        .await?;

        for (field_path, incoming_value) in fields {
            let existing_row = sqlx::query_as::<_, FieldProvenance>(
                r#"
                SELECT profile_id, mixin_key, field_path, value, source, event_id, applied_policy, updated_at
                FROM profile_field_provenance
                WHERE profile_id = $1 AND mixin_key = $2 AND field_path = $3
                "#,
            )
            .bind(profile_id)
            .bind(mixin_key)
            .bind(field_path)
            .fetch_optional(&mut *tx)
            .await?;

            let existing_candidate = existing_row.as_ref().map(|r| FieldCandidate {
                source: r.source.clone(),
                value: r.value.clone(),
                observed_at: r.updated_at,
            });

            let incoming_candidate = FieldCandidate {
                source: source.to_string(),
                value: incoming_value.clone(),
                observed_at: Utc::now(),
            };

            let Decision::Updated(winner) = decide(policy, existing_candidate, incoming_candidate)?
            else {
                continue;
            };

            let strategy_name = format!("{:?}", policy.strategy);

            sqlx::query(
                r#"
                INSERT INTO profile_field_provenance
                    (profile_id, mixin_key, field_path, value, source, event_id, applied_policy, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                ON CONFLICT (profile_id, mixin_key, field_path) DO UPDATE SET
                    value = EXCLUDED.value,
                    source = EXCLUDED.source,
                    event_id = EXCLUDED.event_id,
                    applied_policy = EXCLUDED.applied_policy,
                    updated_at = EXCLUDED.updated_at
                "#,
            )
            .bind(profile_id)
            .bind(mixin_key)
            .bind(field_path)
            .bind(&winner.value)
            .bind(&winner.source)
            .bind(event_id)
            .bind(&strategy_name)
            .bind(winner.observed_at)
            .execute(&mut *tx)
            .await?;

            sqlx::query(
                r#"
                UPDATE profiles
                SET mixins = jsonb_set(
                        jsonb_set(mixins, ARRAY[$2], COALESCE(mixins -> $2, '{}'::jsonb), true),
                        ARRAY[$2, $3],
                        $4,
                        true
                    ),
                    updated_at = now()
                WHERE id = $1
                "#,
            )
            .bind(profile_id)
            .bind(mixin_key)
            .bind(field_path)
            .bind(&winner.value)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }
}
