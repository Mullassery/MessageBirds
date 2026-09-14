use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use mb_identity::IdentityRepo;
use mb_profile::{FieldProvenance, Profile, ProfileRepo};

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
        SELECT id, event_type, schema_name, schema_version, data, received_at, occurred_at
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
