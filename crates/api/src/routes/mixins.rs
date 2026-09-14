use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use std::collections::BTreeMap;

use mb_mixins::{MixinDef, MixinRepo};
use mb_schema_registry::FieldDef;

use crate::error::ApiError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateMixinRequest {
    pub namespace: String,
    pub name: String,
    pub version: String,
    pub fields: BTreeMap<String, FieldDef>,
}

pub async fn create_mixin(
    State(state): State<AppState>,
    Json(req): Json<CreateMixinRequest>,
) -> Result<(StatusCode, Json<MixinDef>), ApiError> {
    let created = state
        .mixins
        .register_custom(&req.namespace, &req.name, &req.version, req.fields)
        .await?;
    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn list_mixins(State(state): State<AppState>) -> Result<Json<Vec<MixinDef>>, ApiError> {
    Ok(Json(state.mixins.list().await?))
}
