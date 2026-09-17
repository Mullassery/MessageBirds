use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use mb_channels::{Channel, ChannelRepo};

use crate::error::ApiError;
use crate::routes::audiences::TenantQuery;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateChannelRequest {
    pub tenant_id: Uuid,
    pub kind: String,
    pub name: String,
    pub config: serde_json::Value,
}

pub async fn create_channel(
    State(state): State<AppState>,
    Json(req): Json<CreateChannelRequest>,
) -> Result<(StatusCode, Json<Channel>), ApiError> {
    let channel = state
        .channels
        .register(req.tenant_id, &req.kind, &req.name, req.config)
        .await?;
    Ok((StatusCode::CREATED, Json(channel)))
}

pub async fn list_channels(
    State(state): State<AppState>,
    Query(q): Query<TenantQuery>,
) -> Result<Json<Vec<Channel>>, ApiError> {
    Ok(Json(state.channels.list(q.tenant_id).await?))
}
