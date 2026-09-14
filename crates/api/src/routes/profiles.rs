use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use mb_identity::{IdentityAuditEntry, IdentityNode, IdentityRepo};
use mb_profile::{FieldProvenance, PersonSignal, Profile, ProfileRepo};

use crate::error::ApiError;
use crate::state::AppState;

#[derive(Serialize)]
pub struct ProfileView {
    #[serde(flatten)]
    pub profile: Profile,
    pub provenance: Vec<FieldProvenance>,
}

async fn load_profile_view(state: &AppState, id: Uuid) -> Result<ProfileView, ApiError> {
    let profile = state
        .profiles
        .get(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("profile {id} not found")))?;
    let provenance = state.profiles.get_provenance(id).await?;
    Ok(ProfileView {
        profile,
        provenance,
    })
}

pub async fn get_profile(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ProfileView>, ApiError> {
    Ok(Json(load_profile_view(&state, id).await?))
}

#[derive(Deserialize)]
pub struct FindByIdentityQuery {
    pub tenant_id: Uuid,
    pub namespace: String,
    pub value: String,
}

/// The client-facing complement to identity resolution: given a claim it
/// already knows (e.g. the `anonymous_id` it generated for itself), find
/// the profile that claim currently resolves to. `resolve_or_create`
/// mints new profile ids as a side effect of ingesting an event; this is
/// the read-only lookup a client uses afterwards.
pub async fn get_profile_by_identity(
    State(state): State<AppState>,
    Query(q): Query<FindByIdentityQuery>,
) -> Result<Json<ProfileView>, ApiError> {
    let profile_id = state
        .identity
        .find_profile_id(q.tenant_id, &q.namespace, &q.value)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!(
                "no profile is linked to {}:{}",
                q.namespace, q.value
            ))
        })?;
    Ok(Json(load_profile_view(&state, profile_id).await?))
}

#[derive(Serialize, sqlx::FromRow)]
pub struct EventSummary {
    pub id: Uuid,
    pub event_type: String,
    pub schema_name: String,
    pub schema_version: String,
    /// The raw identity claims this event carried — the one place an
    /// operator can recover a claim's plaintext value, since the identity
    /// graph (`/profiles/{id}/identity`) only ever stores/returns hashes.
    /// Needed to know what to pass to `POST /profiles/{id}/split`.
    pub identity: serde_json::Value,
    pub data: serde_json::Value,
    pub received_at: DateTime<Utc>,
    pub occurred_at: DateTime<Utc>,
}

pub async fn get_profile_events(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<EventSummary>>, ApiError> {
    let rows = sqlx::query_as::<_, EventSummary>(
        r#"
        SELECT id, event_type, schema_name, schema_version, identity, data, received_at, occurred_at
        FROM events
        WHERE profile_id = $1
        ORDER BY occurred_at DESC
        "#,
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

#[derive(Serialize)]
pub struct IdentityView {
    pub nodes: Vec<IdentityNode>,
    pub audit: Vec<IdentityAuditEntry>,
}

pub async fn get_profile_identity(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<IdentityView>, ApiError> {
    let profile = state
        .profiles
        .get(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("profile {id} not found")))?;
    let nodes = state
        .identity
        .get_identity_graph(profile.tenant_id, id)
        .await?;
    let audit = state
        .identity
        .get_audit_history(profile.tenant_id, id)
        .await?;
    Ok(Json(IdentityView { nodes, audit }))
}

/// Pulls the signals `mb_profile::similarity::score` compares out of a
/// profile's composed mixins. Absent fields just mean that half of the
/// score contributes 0 — see `score`'s doc comment.
fn extract_signal(mixins: &serde_json::Value) -> PersonSignal {
    let person = mixins.get("core/person@1.0");
    let device = mixins.get("core/device@1.0");
    PersonSignal {
        first_name: person
            .and_then(|m| m.get("first_name"))
            .and_then(|v| v.as_str())
            .map(String::from),
        last_name: person
            .and_then(|m| m.get("last_name"))
            .and_then(|v| v.as_str())
            .map(String::from),
        device_id: device
            .and_then(|m| m.get("device_id"))
            .and_then(|v| v.as_str())
            .map(String::from),
    }
}

#[derive(Serialize)]
pub struct MergeSuggestion {
    pub profile_id: Uuid,
    pub score: f64,
}

#[derive(sqlx::FromRow)]
struct ProfileMixinsRow {
    id: Uuid,
    mixins: serde_json::Value,
}

/// Confidence-scored candidates for a manual merge — never merges anything
/// itself. Anchored on a shared `core/device@1.0.device_id`: without that
/// anchor, name-only similarity across an entire tenant is too weak a
/// signal to be useful (many customers share a first name), so a profile
/// with no device id on record gets no suggestions rather than noisy ones.
pub async fn get_merge_suggestions(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<MergeSuggestion>>, ApiError> {
    let profile = state
        .profiles
        .get(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("profile {id} not found")))?;
    let this_signal = extract_signal(&profile.mixins);

    let Some(device_id) = &this_signal.device_id else {
        return Ok(Json(Vec::new()));
    };

    let candidates = sqlx::query_as::<_, ProfileMixinsRow>(
        r#"
        SELECT id, mixins FROM profiles
        WHERE tenant_id = $1 AND id != $2
          AND mixins -> 'core/device@1.0' ->> 'device_id' = $3
        "#,
    )
    .bind(profile.tenant_id)
    .bind(id)
    .bind(device_id)
    .fetch_all(&state.pool)
    .await?;

    let mut suggestions: Vec<MergeSuggestion> = candidates
        .into_iter()
        .map(|c| {
            let candidate_signal = extract_signal(&c.mixins);
            MergeSuggestion {
                profile_id: c.id,
                score: mb_profile::score(&this_signal, &candidate_signal),
            }
        })
        .filter(|s| s.score > 0.3)
        .collect();

    suggestions.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    Ok(Json(suggestions))
}

#[derive(Deserialize)]
pub struct MergeRequest {
    pub from_profile_id: Uuid,
}

pub async fn merge_profile(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<MergeRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if req.from_profile_id == id {
        return Err(ApiError::BadRequest(
            "cannot merge a profile into itself".into(),
        ));
    }
    let profile = state
        .profiles
        .get(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("profile {id} not found")))?;
    state
        .identity
        .merge_profiles(profile.tenant_id, req.from_profile_id, id, "manual-merge")
        .await?;
    Ok(Json(serde_json::json!({
        "merged_from": req.from_profile_id,
        "into": id
    })))
}

#[derive(Deserialize)]
pub struct SplitRequest {
    pub namespace: String,
    pub value: String,
}

pub async fn split_profile(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<SplitRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let profile = state
        .profiles
        .get(id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("profile {id} not found")))?;
    let new_profile_id = state
        .identity
        .split_identity(
            profile.tenant_id,
            id,
            &req.namespace,
            &req.value,
            "manual-split",
        )
        .await?;
    Ok(Json(serde_json::json!({
        "new_profile_id": new_profile_id,
        "note": "only the identity graph link moved; mixin/profile data was not migrated and stays on the original profile"
    })))
}
