use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use std::collections::BTreeMap;
use uuid::Uuid;

use mb_schema_registry::{FieldDef, Schema, SchemaRepo, SchemaStatus};

use crate::error::ApiError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateSchemaRequest {
    pub name: String,
    pub version: String,
    pub fields: BTreeMap<String, FieldDef>,
}

pub async fn create_schema(
    State(state): State<AppState>,
    Json(req): Json<CreateSchemaRequest>,
) -> Result<(StatusCode, Json<Schema>), ApiError> {
    let schema = Schema {
        id: Uuid::nil(),
        name: req.name,
        version: req.version,
        fields: req.fields,
        status: SchemaStatus::Active,
        created_at: chrono::Utc::now(),
    };
    let created = state.schemas.register(schema).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

pub async fn list_schemas(State(state): State<AppState>) -> Result<Json<Vec<Schema>>, ApiError> {
    Ok(Json(state.schemas.list().await?))
}

pub async fn get_schema(
    State(state): State<AppState>,
    Path((name, version)): Path<(String, String)>,
) -> Result<Json<Schema>, ApiError> {
    Ok(Json(state.schemas.get(&name, &version).await?))
}
