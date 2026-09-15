use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use mb_audiences::AudienceRepo;
use mb_connectors::{
    ActivationRepo, ActivationStatus, Destination, DestinationConnector, DestinationRepo,
    WEBHOOK_KIND,
};
use mb_profile::ProfileRepo;

use crate::error::ApiError;
use crate::routes::audiences::TenantQuery;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateDestinationRequest {
    pub tenant_id: Uuid,
    pub kind: String,
    pub name: String,
    pub config: serde_json::Value,
}

pub async fn create_destination(
    State(state): State<AppState>,
    Json(req): Json<CreateDestinationRequest>,
) -> Result<(StatusCode, Json<Destination>), ApiError> {
    if req.kind != WEBHOOK_KIND {
        return Err(ApiError::BadRequest(format!(
            "unsupported destination kind '{}' — only '{WEBHOOK_KIND}' is implemented",
            req.kind
        )));
    }
    let destination = state
        .connectors
        .register(req.tenant_id, &req.kind, &req.name, req.config)
        .await?;
    Ok((StatusCode::CREATED, Json(destination)))
}

pub async fn list_destinations(
    State(state): State<AppState>,
    Query(q): Query<TenantQuery>,
) -> Result<Json<Vec<Destination>>, ApiError> {
    Ok(Json(state.connectors.list(q.tenant_id).await?))
}

#[derive(Deserialize)]
pub struct ActivateRequest {
    pub tenant_id: Uuid,
    pub destination_id: Uuid,
}

#[derive(Serialize)]
pub struct ActivationSummary {
    pub sent: usize,
    pub failed: usize,
}

/// Audience → destination capability check → send → audit record
/// (Section 60), minus the governance/consent steps that aren't
/// implemented yet. Runs synchronously over the audience's current
/// members — fine at dev-mode member counts, a real queue is future work.
pub async fn activate_audience(
    State(state): State<AppState>,
    Path(audience_id): Path<Uuid>,
    Json(req): Json<ActivateRequest>,
) -> Result<Json<ActivationSummary>, ApiError> {
    let destination = state
        .connectors
        .get(req.tenant_id, req.destination_id)
        .await?;

    if destination.kind != WEBHOOK_KIND {
        return Err(ApiError::BadRequest(format!(
            "destination kind '{}' has no connector implementation",
            destination.kind
        )));
    }

    let members = state.audiences.get_members(audience_id).await?;

    let mut sent = 0usize;
    let mut failed = 0usize;

    for member in members {
        let payload = match state.profiles.get(member.profile_id).await? {
            Some(profile) => serde_json::json!({
                "profile_id": profile.id,
                "mixins": profile.mixins,
            }),
            None => continue,
        };

        let result = state.webhook.send(&destination.config, &payload).await;
        let (status, detail) = match &result {
            Ok(()) => {
                sent += 1;
                (ActivationStatus::Sent, None)
            }
            Err(e) => {
                failed += 1;
                (ActivationStatus::Failed, Some(e.to_string()))
            }
        };

        state
            .connectors
            .log(
                req.tenant_id,
                audience_id,
                destination.id,
                member.profile_id,
                status,
                detail,
            )
            .await?;
    }

    Ok(Json(ActivationSummary { sent, failed }))
}
