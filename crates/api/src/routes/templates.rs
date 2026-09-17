use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use mb_templates::{MessageTemplate, TemplateRepo};

use crate::error::ApiError;
use crate::routes::audiences::TenantQuery;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateTemplateRequest {
    pub tenant_id: Uuid,
    pub channel_kind: String,
    pub name: String,
    #[serde(default)]
    pub subject: Option<String>,
    pub body: String,
}

pub async fn create_template(
    State(state): State<AppState>,
    Json(req): Json<CreateTemplateRequest>,
) -> Result<(StatusCode, Json<MessageTemplate>), ApiError> {
    let template = state
        .templates
        .register(
            req.tenant_id,
            &req.channel_kind,
            &req.name,
            req.subject,
            req.body,
        )
        .await?;
    Ok((StatusCode::CREATED, Json(template)))
}

pub async fn list_templates(
    State(state): State<AppState>,
    Query(q): Query<TenantQuery>,
) -> Result<Json<Vec<MessageTemplate>>, ApiError> {
    Ok(Json(state.templates.list(q.tenant_id).await?))
}
