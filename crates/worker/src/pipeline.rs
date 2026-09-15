use std::sync::Arc;

use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use mb_audiences::{AudienceError, AudienceRepo};
use mb_core::{EventEnvelope, MixinRef};
use mb_identity::{IdentityError, IdentityRepo};
use mb_merge_policy::{MergePolicyRepo, MergePolicyRepoError};
use mb_mixins::{validate_mixin_fields, MixinError, MixinRepo};
use mb_profile::{ProfileError, ProfileRepo};
use mb_schema_registry::{validate_payload, SchemaError, SchemaRepo};

use crate::identity_mixin::derive_identity_mixin_fields;
use crate::persist::{persist_dead_letter, persist_event};

#[derive(Debug, Error)]
pub enum WorkerError {
    #[error(transparent)]
    Schema(#[from] SchemaError),
    #[error(transparent)]
    Mixin(#[from] MixinError),
    #[error(transparent)]
    Identity(#[from] IdentityError),
    #[error(transparent)]
    MergePolicy(#[from] MergePolicyRepoError),
    #[error(transparent)]
    Profile(#[from] ProfileError),
    #[error(transparent)]
    Audience(#[from] AudienceError),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

#[derive(Debug, PartialEq)]
pub enum Outcome {
    Accepted { profile_id: Uuid },
    Rejected { reason: String },
}

pub struct Pipeline {
    pub pool: PgPool,
    pub schemas: Arc<dyn SchemaRepo>,
    pub mixins: Arc<dyn MixinRepo>,
    pub identity: Arc<dyn IdentityRepo>,
    pub merge_policies: Arc<dyn MergePolicyRepo>,
    pub profiles: Arc<dyn ProfileRepo>,
    pub audiences: Arc<dyn AudienceRepo>,
}

impl Pipeline {
    /// Runs one event through EVENT -> SCHEMA -> MIXIN -> IDENTITY -> MERGE
    /// POLICY -> PROFILE. Returns `Ok(Rejected)` (not `Err`) for bad data —
    /// that's a successful DLQ write, and the caller should still commit
    /// the Kafka offset so a permanently-invalid event isn't retried
    /// forever. `Err` means an infrastructure failure; the caller should
    /// leave the offset uncommitted so the event is redelivered.
    pub async fn process(&self, envelope: &EventEnvelope) -> Result<Outcome, WorkerError> {
        let schema = match self
            .schemas
            .get(&envelope.schema.name, &envelope.schema.version)
            .await
        {
            Ok(schema) => schema,
            Err(SchemaError::NotFound { .. }) => {
                return self
                    .reject(envelope, "no matching schema is registered".into())
                    .await;
            }
            Err(e) => return Err(WorkerError::Schema(e)),
        };

        let violations = validate_payload(&schema, &envelope.data);
        if !violations.is_empty() {
            let reason = format!(
                "schema validation failed: {}",
                violations
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join("; ")
            );
            return self.reject(envelope, reason).await;
        }

        let resolved = self
            .identity
            .resolve_or_create(
                envelope.tenant_id.0,
                &envelope.identity,
                &envelope.source.name,
            )
            .await?;
        let profile_id = resolved.profile_id;

        let policy = self.merge_policies.get_active(envelope.tenant_id.0).await?;

        let identity_fields = derive_identity_mixin_fields(&envelope.identity);
        if identity_fields.as_object().is_some_and(|o| !o.is_empty()) {
            self.profiles
                .apply_mixin_update(
                    envelope.tenant_id.0,
                    profile_id,
                    "core/identity@1.0",
                    &identity_fields,
                    &envelope.source.name,
                    envelope.event_id,
                    &policy,
                )
                .await?;
        }

        if let Some(updates) = envelope
            .context
            .get("profile_updates")
            .and_then(|v| v.as_object())
        {
            for (mixin_key, data) in updates {
                let Ok(mixin_ref) = MixinRef::parse(mixin_key) else {
                    return self
                        .reject(envelope, format!("invalid mixin reference '{mixin_key}'"))
                        .await;
                };

                let mixin_def = match self
                    .mixins
                    .get(&mixin_ref.namespace, &mixin_ref.name, &mixin_ref.version)
                    .await
                {
                    Ok(def) => def,
                    Err(MixinError::NotFound { .. }) => {
                        return self
                            .reject(envelope, format!("mixin '{mixin_key}' is not registered"))
                            .await;
                    }
                    Err(e) => return Err(WorkerError::Mixin(e)),
                };

                let violations = validate_mixin_fields(&mixin_def, data);
                if !violations.is_empty() {
                    let reason = format!(
                        "mixin '{mixin_key}' validation failed: {}",
                        violations
                            .iter()
                            .map(|v| v.to_string())
                            .collect::<Vec<_>>()
                            .join("; ")
                    );
                    return self.reject(envelope, reason).await;
                }

                self.profiles
                    .apply_mixin_update(
                        envelope.tenant_id.0,
                        profile_id,
                        mixin_key,
                        data,
                        &envelope.source.name,
                        envelope.event_id,
                        &policy,
                    )
                    .await?;
            }
        }

        // Must happen before audience evaluation: an `Event` condition
        // queries the `events` table, and this event should be visible to
        // its own audience check (e.g. "entered on the Nth qualifying
        // event," not "entered starting from the N+1th").
        persist_event(&self.pool, envelope, profile_id).await?;

        // `apply_mixin_update` (and therefore the `profiles` row) is only
        // reached if the event carried an identity-mixin-mappable claim or
        // a `profile_updates` block — an event with e.g. only a
        // non-primary `email` claim and no `profile_updates` can validly
        // reach here without a profile row existing yet, so this checks
        // rather than assumes.
        if let Some(profile) = self.profiles.get(profile_id).await? {
            self.audiences
                .evaluate_and_sync_membership(envelope.tenant_id.0, profile_id, &profile.mixins)
                .await?;
        }

        Ok(Outcome::Accepted { profile_id })
    }

    async fn reject(
        &self,
        envelope: &EventEnvelope,
        reason: String,
    ) -> Result<Outcome, WorkerError> {
        persist_dead_letter(&self.pool, envelope, &reason).await?;
        Ok(Outcome::Rejected { reason })
    }
}
