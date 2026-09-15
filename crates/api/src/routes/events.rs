use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use mb_core::EventEnvelope;
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;

/// Publishes to Kafka and returns immediately. Deep schema/mixin
/// validation happens in the worker, asynchronously — this endpoint only
/// checks the envelope is structurally well-formed.
pub async fn post_event(
    State(state): State<AppState>,
    Json(envelope): Json<EventEnvelope>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    envelope
        .validate_shape()
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    state
        .producer
        .publish(mb_events::RAW_EVENTS_TOPIC, &envelope)
        .await?;

    Ok((
        StatusCode::ACCEPTED,
        Json(json!({ "event_id": envelope.event_id })),
    ))
}

#[derive(Serialize, sqlx::FromRow)]
pub struct EventDetail {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub event_type: String,
    pub schema_name: String,
    pub schema_version: String,
    pub identity: serde_json::Value,
    pub context: serde_json::Value,
    pub data: serde_json::Value,
    pub profile_id: Uuid,
    pub received_at: DateTime<Utc>,
    pub occurred_at: DateTime<Utc>,
}

/// Single-event lookup — used by the field lineage view to show the
/// source event behind a profile field's winning value.
pub async fn get_event(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<EventDetail>, ApiError> {
    let row = sqlx::query_as::<_, EventDetail>(
        r#"
        SELECT id, tenant_id, event_type, schema_name, schema_version, identity, context, data, profile_id, received_at, occurred_at
        FROM events
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("event {id} not found")))?;
    Ok(Json(row))
}
