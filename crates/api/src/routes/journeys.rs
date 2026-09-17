use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use mb_journeys::{
    ContactPolicy, ContactPolicyRepo, JourneyDefinition, JourneyRepo, JourneyRun, JourneyRunEvent,
    Node, Trigger,
};

use crate::error::ApiError;
use crate::routes::audiences::TenantQuery;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateJourneyRequest {
    pub tenant_id: Uuid,
    pub name: String,
    pub trigger: Trigger,
    pub nodes: Vec<Node>,
    pub entry_node: String,
}

pub async fn create_journey(
    State(state): State<AppState>,
    Json(req): Json<CreateJourneyRequest>,
) -> Result<(StatusCode, Json<JourneyDefinition>), ApiError> {
    let journey = state
        .journeys
        .register(
            req.tenant_id,
            &req.name,
            req.trigger,
            req.nodes,
            &req.entry_node,
        )
        .await?;
    Ok((StatusCode::CREATED, Json(journey)))
}

pub async fn list_journeys(
    State(state): State<AppState>,
    Query(q): Query<TenantQuery>,
) -> Result<Json<Vec<JourneyDefinition>>, ApiError> {
    Ok(Json(state.journeys.list(q.tenant_id).await?))
}

pub async fn get_journey(
    State(state): State<AppState>,
    Query(q): Query<TenantQuery>,
    Path(id): Path<Uuid>,
) -> Result<Json<JourneyDefinition>, ApiError> {
    Ok(Json(state.journeys.get(q.tenant_id, id).await?))
}

pub async fn get_journey_runs(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<JourneyRun>>, ApiError> {
    Ok(Json(state.journeys.get_runs(id).await?))
}

#[derive(Deserialize)]
pub struct StartRunRequest {
    pub profile_id: Uuid,
}

/// Manual enrollment — useful for testing, and a legitimate operational
/// escape hatch (not just a test hook): an operator may want to drop a
/// specific customer into a journey without waiting for the audience
/// trigger to fire naturally.
pub async fn start_journey(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<StartRunRequest>,
) -> Result<(StatusCode, Json<JourneyRun>), ApiError> {
    let run = state.journeys.start_run(id, req.profile_id).await?;
    Ok((StatusCode::CREATED, Json(run)))
}

pub async fn get_run_events(
    State(state): State<AppState>,
    Path(run_id): Path<Uuid>,
) -> Result<Json<Vec<JourneyRunEvent>>, ApiError> {
    Ok(Json(state.journeys.get_run_events(run_id).await?))
}

pub async fn get_profile_journeys(
    State(state): State<AppState>,
    Query(q): Query<TenantQuery>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<JourneyRun>>, ApiError> {
    Ok(Json(
        state.journeys.get_profile_runs(q.tenant_id, id).await?,
    ))
}

#[derive(Deserialize)]
pub struct CreateContactPolicyRequest {
    pub tenant_id: Uuid,
    pub max_messages: i32,
    pub window_days: i32,
    #[serde(default)]
    pub channel_id: Option<Uuid>,
}

pub async fn create_contact_policy(
    State(state): State<AppState>,
    Json(req): Json<CreateContactPolicyRequest>,
) -> Result<(StatusCode, Json<ContactPolicy>), ApiError> {
    let policy = state
        .journeys
        .register_policy(
            req.tenant_id,
            req.max_messages,
            req.window_days,
            req.channel_id,
        )
        .await?;
    Ok((StatusCode::CREATED, Json(policy)))
}

pub async fn list_contact_policies(
    State(state): State<AppState>,
    Query(q): Query<TenantQuery>,
) -> Result<Json<Vec<ContactPolicy>>, ApiError> {
    Ok(Json(state.journeys.list_policies(q.tenant_id).await?))
}
