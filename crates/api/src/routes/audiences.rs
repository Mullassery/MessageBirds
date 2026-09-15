use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use mb_audiences::{AudienceDefinition, AudienceRepo, Condition, Membership};

use crate::error::ApiError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateAudienceRequest {
    pub tenant_id: Uuid,
    pub name: String,
    pub conditions: Condition,
}

pub async fn create_audience(
    State(state): State<AppState>,
    Json(req): Json<CreateAudienceRequest>,
) -> Result<(StatusCode, Json<AudienceDefinition>), ApiError> {
    let audience = state
        .audiences
        .register(req.tenant_id, &req.name, req.conditions)
        .await?;
    Ok((StatusCode::CREATED, Json(audience)))
}

#[derive(Deserialize)]
pub struct TenantQuery {
    pub tenant_id: Uuid,
}

pub async fn list_audiences(
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<TenantQuery>,
) -> Result<Json<Vec<AudienceDefinition>>, ApiError> {
    Ok(Json(state.audiences.list(q.tenant_id).await?))
}

pub async fn get_audience(
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<TenantQuery>,
    Path(id): Path<Uuid>,
) -> Result<Json<AudienceDefinition>, ApiError> {
    Ok(Json(state.audiences.get(q.tenant_id, id).await?))
}

pub async fn get_audience_members(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<Membership>>, ApiError> {
    Ok(Json(state.audiences.get_members(id).await?))
}
