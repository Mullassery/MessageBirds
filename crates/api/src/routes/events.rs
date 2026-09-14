use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use mb_core::EventEnvelope;
use serde_json::json;

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
