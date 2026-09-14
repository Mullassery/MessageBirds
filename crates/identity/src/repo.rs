use async_trait::async_trait;
use mb_core::IdentityRef;
use sqlx::{PgConnection, PgPool};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

use crate::hash::hash_value;
use crate::model::{IdentityAuditEntry, IdentityNode, ResolvedIdentity};
use crate::resolve::{pick_canonical, profiles_to_merge};

#[derive(Debug, Error)]
pub enum IdentityError {
    #[error("identity claim {namespace}:{value} is not currently linked to profile {profile_id}")]
    NotLinked {
        profile_id: Uuid,
        namespace: String,
        value: String,
    },
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

#[async_trait]
pub trait IdentityRepo: Send + Sync {
    /// Resolves a set of identity claims from one event against the
    /// identity graph, creating a new profile id if none of them have
    /// been seen before, and merging profiles together if the claims
    /// disagree about which existing profile they belong to.
    async fn resolve_or_create(
        &self,
        tenant_id: Uuid,
        claims: &[IdentityRef],
        source: &str,
    ) -> Result<ResolvedIdentity, IdentityError>;

    /// Looks up the profile currently linked to one identity claim, without
    /// mutating the graph. This is how a client that only ever sends
    /// anonymous/device-scoped identifiers finds "its" profile id after
    /// the fact — `resolve_or_create` mints ids, this just reads.
    async fn find_profile_id(
        &self,
        tenant_id: Uuid,
        namespace: &str,
        value: &str,
    ) -> Result<Option<Uuid>, IdentityError>;

    /// Explicitly merges `from` into `into` — the manual counterpart to the
    /// automatic merge `resolve_or_create` performs when an event's claims
    /// disagree. Used to confirm a probabilistic merge suggestion.
    async fn merge_profiles(
        &self,
        tenant_id: Uuid,
        from: Uuid,
        into: Uuid,
        source: &str,
    ) -> Result<(), IdentityError>;

    /// Unlinks one identity claim from `profile_id` and mints it a fresh
    /// profile id going forward. This only moves the identity graph edge —
    /// it does **not** migrate any mixin/profile data, which by this point
    /// may already be commingled with data from other claims on the same
    /// profile and can't be reliably attributed back to just this claim.
    async fn split_identity(
        &self,
        tenant_id: Uuid,
        profile_id: Uuid,
        namespace: &str,
        value: &str,
        source: &str,
    ) -> Result<Uuid, IdentityError>;

    /// Every identity claim currently linked to a profile.
    async fn get_identity_graph(
        &self,
        tenant_id: Uuid,
        profile_id: Uuid,
    ) -> Result<Vec<IdentityNode>, IdentityError>;

    /// The append-only audit trail touching a profile, on either side of a
    /// merge or split (i.e. as the current `profile_id` or as a
    /// `previous_profile_id`).
    async fn get_audit_history(
        &self,
        tenant_id: Uuid,
        profile_id: Uuid,
    ) -> Result<Vec<IdentityAuditEntry>, IdentityError>;
}

pub struct PgIdentityRepo {
    pool: PgPool,
}

impl PgIdentityRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Re-points every identity node owned by `from` to `into` and records the
/// merge. Shared by the automatic merge inside `resolve_or_create` and the
/// explicit `merge_profiles`.
async fn merge_one(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    from: Uuid,
    into: Uuid,
    source: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE identity_nodes SET profile_id = $1 WHERE tenant_id = $2 AND profile_id = $3",
    )
    .bind(into)
    .bind(tenant_id)
    .bind(from)
    .execute(&mut *conn)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO identity_audit (id, tenant_id, kind, namespace, value_hash, profile_id, previous_profile_id, source, created_at)
        VALUES ($1, $2, 'merged', NULL, NULL, $3, $4, $5, now())
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(tenant_id)
    .bind(into)
    .bind(from)
    .bind(source)
    .execute(&mut *conn)
    .await?;

    Ok(())
}

#[async_trait]
impl IdentityRepo for PgIdentityRepo {
    async fn resolve_or_create(
        &self,
        tenant_id: Uuid,
        claims: &[IdentityRef],
        source: &str,
    ) -> Result<ResolvedIdentity, IdentityError> {
        let mut tx = self.pool.begin().await?;

        let hashes: Vec<String> = claims
            .iter()
            .map(|c| hash_value(&c.namespace, &c.value))
            .collect();
        let namespaces: Vec<String> = claims.iter().map(|c| c.namespace.clone()).collect();

        let rows: Vec<(String, String, Uuid)> = sqlx::query_as(
            r#"
            SELECT namespace, value_hash, profile_id
            FROM identity_nodes
            WHERE tenant_id = $1
              AND (namespace, value_hash) IN (
                  SELECT * FROM UNNEST($2::text[], $3::text[])
              )
            "#,
        )
        .bind(tenant_id)
        .bind(&namespaces)
        .bind(&hashes)
        .fetch_all(&mut *tx)
        .await?;

        let mut lookup: HashMap<(String, String), Uuid> = HashMap::new();
        for (ns, h, pid) in rows {
            lookup.insert((ns, h), pid);
        }

        let existing: Vec<Option<Uuid>> = namespaces
            .iter()
            .zip(&hashes)
            .map(|(ns, h)| lookup.get(&(ns.clone(), h.clone())).copied())
            .collect();

        let canonical = pick_canonical(claims, &existing).unwrap_or_else(Uuid::new_v4);
        let merged = profiles_to_merge(canonical, &existing);

        for merged_id in &merged {
            merge_one(&mut tx, tenant_id, *merged_id, canonical, source).await?;
        }

        for (claim, hash) in claims.iter().zip(&hashes) {
            let inserted: bool = sqlx::query_scalar(
                r#"
                INSERT INTO identity_nodes
                    (id, tenant_id, namespace, value_hash, profile_id, confidence, source, first_seen_at, last_seen_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, now(), now())
                ON CONFLICT (tenant_id, namespace, value_hash)
                DO UPDATE SET
                    last_seen_at = now(),
                    confidence = GREATEST(identity_nodes.confidence, EXCLUDED.confidence),
                    source = EXCLUDED.source
                RETURNING (xmax = 0)
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(tenant_id)
            .bind(&claim.namespace)
            .bind(hash)
            .bind(canonical)
            .bind(claim.confidence)
            .bind(&claim.source)
            .fetch_one(&mut *tx)
            .await?;

            if inserted {
                sqlx::query(
                    r#"
                    INSERT INTO identity_audit (id, tenant_id, kind, namespace, value_hash, profile_id, previous_profile_id, source, created_at)
                    VALUES ($1, $2, 'linked', $3, $4, $5, NULL, $6, now())
                    "#,
                )
                .bind(Uuid::new_v4())
                .bind(tenant_id)
                .bind(&claim.namespace)
                .bind(hash)
                .bind(canonical)
                .bind(source)
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;

        Ok(ResolvedIdentity {
            profile_id: canonical,
            merged_profile_ids: merged,
        })
    }

    async fn find_profile_id(
        &self,
        tenant_id: Uuid,
        namespace: &str,
        value: &str,
    ) -> Result<Option<Uuid>, IdentityError> {
        let hash = hash_value(namespace, value);
        let profile_id: Option<Uuid> = sqlx::query_scalar(
            "SELECT profile_id FROM identity_nodes WHERE tenant_id = $1 AND namespace = $2 AND value_hash = $3",
        )
        .bind(tenant_id)
        .bind(namespace)
        .bind(hash)
        .fetch_optional(&self.pool)
        .await?;
        Ok(profile_id)
    }

    async fn merge_profiles(
        &self,
        tenant_id: Uuid,
        from: Uuid,
        into: Uuid,
        source: &str,
    ) -> Result<(), IdentityError> {
        let mut tx = self.pool.begin().await?;
        merge_one(&mut tx, tenant_id, from, into, source).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn split_identity(
        &self,
        tenant_id: Uuid,
        profile_id: Uuid,
        namespace: &str,
        value: &str,
        source: &str,
    ) -> Result<Uuid, IdentityError> {
        let hash = hash_value(namespace, value);
        let mut tx = self.pool.begin().await?;

        let current_owner: Option<Uuid> = sqlx::query_scalar(
            "SELECT profile_id FROM identity_nodes WHERE tenant_id = $1 AND namespace = $2 AND value_hash = $3",
        )
        .bind(tenant_id)
        .bind(namespace)
        .bind(&hash)
        .fetch_optional(&mut *tx)
        .await?;

        if current_owner != Some(profile_id) {
            return Err(IdentityError::NotLinked {
                profile_id,
                namespace: namespace.to_string(),
                value: value.to_string(),
            });
        }

        let new_profile_id = Uuid::new_v4();
        sqlx::query(
            "UPDATE identity_nodes SET profile_id = $1 WHERE tenant_id = $2 AND namespace = $3 AND value_hash = $4",
        )
        .bind(new_profile_id)
        .bind(tenant_id)
        .bind(namespace)
        .bind(&hash)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO identity_audit (id, tenant_id, kind, namespace, value_hash, profile_id, previous_profile_id, source, created_at)
            VALUES ($1, $2, 'split', $3, $4, $5, $6, $7, now())
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(tenant_id)
        .bind(namespace)
        .bind(&hash)
        .bind(new_profile_id)
        .bind(profile_id)
        .bind(source)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(new_profile_id)
    }

    async fn get_identity_graph(
        &self,
        tenant_id: Uuid,
        profile_id: Uuid,
    ) -> Result<Vec<IdentityNode>, IdentityError> {
        let rows = sqlx::query_as::<_, IdentityNode>(
            r#"
            SELECT id, tenant_id, namespace, value_hash, profile_id, confidence, source, first_seen_at, last_seen_at
            FROM identity_nodes
            WHERE tenant_id = $1 AND profile_id = $2
            ORDER BY first_seen_at
            "#,
        )
        .bind(tenant_id)
        .bind(profile_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn get_audit_history(
        &self,
        tenant_id: Uuid,
        profile_id: Uuid,
    ) -> Result<Vec<IdentityAuditEntry>, IdentityError> {
        let rows = sqlx::query_as::<_, IdentityAuditEntry>(
            r#"
            SELECT id, tenant_id, kind, namespace, value_hash, profile_id, previous_profile_id, source, created_at
            FROM identity_audit
            WHERE tenant_id = $1 AND (profile_id = $2 OR previous_profile_id = $2)
            ORDER BY created_at
            "#,
        )
        .bind(tenant_id)
        .bind(profile_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}
