use async_trait::async_trait;
use mb_core::IdentityRef;
use sqlx::PgPool;
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

use crate::hash::hash_value;
use crate::model::ResolvedIdentity;
use crate::resolve::{pick_canonical, profiles_to_merge};

#[derive(Debug, Error)]
pub enum IdentityError {
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
}

pub struct PgIdentityRepo {
    pool: PgPool,
}

impl PgIdentityRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
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

        if !merged.is_empty() {
            sqlx::query(
                "UPDATE identity_nodes SET profile_id = $1 WHERE tenant_id = $2 AND profile_id = ANY($3)",
            )
            .bind(canonical)
            .bind(tenant_id)
            .bind(&merged)
            .execute(&mut *tx)
            .await?;

            for merged_id in &merged {
                sqlx::query(
                    r#"
                    INSERT INTO identity_audit (id, tenant_id, kind, namespace, value_hash, profile_id, previous_profile_id, source, created_at)
                    VALUES ($1, $2, 'merged', NULL, NULL, $3, $4, $5, now())
                    "#,
                )
                .bind(Uuid::new_v4())
                .bind(tenant_id)
                .bind(canonical)
                .bind(merged_id)
                .bind(source)
                .execute(&mut *tx)
                .await?;
            }
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
}
