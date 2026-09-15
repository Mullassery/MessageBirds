use axum::extract::{Query, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::error::ApiError;
use crate::routes::audiences::TenantQuery;
use crate::state::AppState;

#[derive(Serialize, sqlx::FromRow)]
pub struct DeadLetterEventView {
    pub id: Uuid,
    pub event_id: Uuid,
    pub schema_name: String,
    pub schema_version: String,
    pub source: String,
    pub reason: String,
    pub raw_payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

pub async fn list_dead_letter_events(
    State(state): State<AppState>,
    Query(q): Query<TenantQuery>,
) -> Result<Json<Vec<DeadLetterEventView>>, ApiError> {
    let rows = sqlx::query_as::<_, DeadLetterEventView>(
        r#"
        SELECT id, event_id, schema_name, schema_version, source, reason, raw_payload, created_at
        FROM dead_letter_events
        WHERE tenant_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(q.tenant_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}
