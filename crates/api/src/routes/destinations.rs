use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use uuid::Uuid;

use mb_audiences::AudienceRepo;
use mb_connectors::{
    ActivationRepo, ActivationStatus, Destination, DestinationConnector, DestinationRepo,
    ExportRow, PyReverseEtlExportRepo, PYREVERSEETL_KIND, WEBHOOK_KIND,
};
use mb_governance::{consent_purpose_for_action, ConsentRepo, PolicyRepo, Reason};
use mb_profile::ProfileRepo;

use crate::error::ApiError;
use crate::labels::{flatten_labels, profile_field_labels};
use crate::routes::audiences::TenantQuery;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateDestinationRequest {
    pub tenant_id: Uuid,
    pub kind: String,
    pub name: String,
    pub config: serde_json::Value,
    #[serde(default)]
    pub supported_actions: Vec<String>,
}

pub async fn create_destination(
    State(state): State<AppState>,
    Json(req): Json<CreateDestinationRequest>,
) -> Result<(StatusCode, Json<Destination>), ApiError> {
    if req.kind != WEBHOOK_KIND && req.kind != PYREVERSEETL_KIND {
        return Err(ApiError::BadRequest(format!(
            "unsupported destination kind '{}' — only '{WEBHOOK_KIND}' and '{PYREVERSEETL_KIND}' are implemented",
            req.kind
        )));
    }
    let destination = state
        .connectors
        .register(
            req.tenant_id,
            &req.kind,
            &req.name,
            req.config,
            req.supported_actions,
        )
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
    pub action: String,
}

#[derive(Serialize)]
pub struct BlockedProfile {
    pub profile_id: Uuid,
    pub reasons: Vec<Reason>,
}

#[derive(Serialize)]
pub struct ActivationSummary {
    pub sent: usize,
    pub failed: usize,
    pub blocked: Vec<BlockedProfile>,
}

/// Audience → governance (label policy + consent) → destination capability
/// → transform → send → audit record (Sections 15, 17, 60). Checked
/// per-profile, not just once for the whole audience, since consent is
/// genuinely per-profile — a blocked profile is skipped and explained
/// (Section 16), not silently dropped or silently sent anyway. Runs
/// synchronously over the audience's current members — fine at dev-mode
/// member counts, a real queue is future work.
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
    let consent_purpose = consent_purpose_for_action(&req.action);

    let mut sent = 0usize;
    let mut failed = 0usize;
    let mut blocked = Vec::new();

    for member in members {
        let Some(profile) = state.profiles.get(member.profile_id).await? else {
            continue;
        };

        let mut reasons = Vec::new();

        if !destination
            .supported_actions
            .iter()
            .any(|a| a == &req.action)
        {
            reasons.push(Reason::DestinationCapability {
                action: req.action.clone(),
            });
        }

        let field_labels = profile_field_labels(&state, &profile).await?;
        let labels = flatten_labels(&field_labels);
        reasons.extend(
            state
                .governance
                .evaluate_labels(req.tenant_id, &labels, &req.action)
                .await?,
        );

        if let Some(purpose) = consent_purpose {
            let has_consent = state
                .governance
                .get_current(req.tenant_id, profile.id, purpose)
                .await?
                .map(|s| s.granted)
                .unwrap_or(false);
            if !has_consent {
                reasons.push(Reason::ConsentMissing {
                    purpose: purpose.to_string(),
                });
            }
        }

        if !reasons.is_empty() {
            let detail = serde_json::to_string(&reasons).ok();
            state
                .connectors
                .log(
                    req.tenant_id,
                    audience_id,
                    destination.id,
                    profile.id,
                    ActivationStatus::Blocked,
                    detail,
                )
                .await?;
            blocked.push(BlockedProfile {
                profile_id: profile.id,
                reasons,
            });
            continue;
        }

        let payload = serde_json::json!({
            "profile_id": profile.id,
            "mixins": profile.mixins,
        });

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
                profile.id,
                status,
                detail,
            )
            .await?;
    }

    Ok(Json(ActivationSummary {
        sent,
        failed,
        blocked,
    }))
}

#[derive(Deserialize)]
pub struct ActivateViaReverseEtlRequest {
    pub tenant_id: Uuid,
    pub destination_id: Uuid,
    pub action: String,
}

#[derive(Serialize)]
pub struct ReverseEtlActivationSummary {
    pub exported: usize,
    pub blocked: Vec<BlockedProfile>,
    /// Whatever `pyreverseetl execute-activation` printed to stdout,
    /// parsed as JSON if it was (its real result, e.g. `rows_synced`) —
    /// falls back to the raw stdout/stderr text if it wasn't.
    pub pyreverseetl_result: serde_json::Value,
}

/// Bulk activation through PyReverseETL (Phase 6, Track E) — a separate
/// code path from `activate_audience` above, not a variant of it.
/// PyReverseETL is a bulk source-table -> destination sync tool (its CLI
/// is `create-workflow` + `create-activation` + `execute-activation`,
/// driven by its own real Rust sync engine), not a per-record API, so it
/// doesn't fit `DestinationConnector::send`'s per-profile loop.
///
/// Consent is checked per profile, same as `activate_audience` — a
/// profile without the required consent is excluded from the export and
/// logged `Blocked`, not silently included. Field-level label policy
/// (`evaluate_labels`) is **not** re-run on this path this phase — a
/// disclosed gap, see `docs/ARCHITECTURE.md`'s Phase 6 section, not a
/// silent skip.
pub async fn activate_audience_via_reverse_etl(
    State(state): State<AppState>,
    Path(audience_id): Path<Uuid>,
    Json(req): Json<ActivateViaReverseEtlRequest>,
) -> Result<Json<ReverseEtlActivationSummary>, ApiError> {
    let destination = state
        .connectors
        .get(req.tenant_id, req.destination_id)
        .await?;

    if destination.kind != PYREVERSEETL_KIND {
        return Err(ApiError::BadRequest(format!(
            "destination kind '{}' is not a '{PYREVERSEETL_KIND}' destination",
            destination.kind
        )));
    }

    let config = &destination.config;
    let str_field = |key: &str| -> Result<String, ApiError> {
        config
            .get(key)
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| ApiError::BadRequest(format!("destination config missing '{key}'")))
    };
    let activation_id = str_field("pyreverseetl_activation_id")?;
    let binary = config
        .get("pyreverseetl_binary")
        .and_then(|v| v.as_str())
        .unwrap_or("pyreverseetl")
        .to_string();
    let state_path = config
        .get("pyreverseetl_state_path")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let members = state.audiences.get_members(audience_id).await?;
    let consent_purpose = consent_purpose_for_action(&req.action);

    let mut export_rows = Vec::new();
    let mut blocked = Vec::new();

    for member in members {
        let Some(profile) = state.profiles.get(member.profile_id).await? else {
            continue;
        };

        let mut reasons = Vec::new();

        if !destination
            .supported_actions
            .iter()
            .any(|a| a == &req.action)
        {
            reasons.push(Reason::DestinationCapability {
                action: req.action.clone(),
            });
        }

        if let Some(purpose) = consent_purpose {
            let has_consent = state
                .governance
                .get_current(req.tenant_id, profile.id, purpose)
                .await?
                .map(|s| s.granted)
                .unwrap_or(false);
            if !has_consent {
                reasons.push(Reason::ConsentMissing {
                    purpose: purpose.to_string(),
                });
            }
        }

        if !reasons.is_empty() {
            let detail = serde_json::to_string(&reasons).ok();
            state
                .connectors
                .log(
                    req.tenant_id,
                    audience_id,
                    destination.id,
                    profile.id,
                    ActivationStatus::Blocked,
                    detail,
                )
                .await?;
            blocked.push(BlockedProfile {
                profile_id: profile.id,
                reasons,
            });
            continue;
        }

        let field = |mixin: &str, name: &str| -> Option<String> {
            profile
                .mixins
                .get(mixin)
                .and_then(|m| m.get(name))
                .and_then(|v| v.as_str())
                .map(str::to_string)
        };

        export_rows.push(ExportRow {
            profile_id: profile.id,
            email: field("core/contact@1.0", "email"),
            first_name: field("core/person@1.0", "first_name"),
            last_name: field("core/person@1.0", "last_name"),
            phone: field("core/contact@1.0", "phone"),
        });
    }

    state
        .connectors
        .replace_staging(req.tenant_id, destination.id, export_rows.clone())
        .await?;

    let mut command = Command::new(&binary);
    command.arg("execute-activation").arg(&activation_id);
    if let Some(path) = &state_path {
        command.env("PYREVERSEETL_STATE_PATH", path);
    }
    let output = command
        .output()
        .await
        .map_err(|e| ApiError::Internal(format!("failed to run '{binary}': {e}")))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let pyreverseetl_result: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|_| serde_json::json!({ "stdout": stdout, "stderr": stderr }));

    let profile_status = if output.status.success() {
        ActivationStatus::Sent
    } else {
        ActivationStatus::Failed
    };
    let detail = serde_json::to_string(&pyreverseetl_result).ok();
    for row in &export_rows {
        state
            .connectors
            .log(
                req.tenant_id,
                audience_id,
                destination.id,
                row.profile_id,
                profile_status,
                detail.clone(),
            )
            .await?;
    }

    Ok(Json(ReverseEtlActivationSummary {
        exported: export_rows.len(),
        blocked,
        pyreverseetl_result,
    }))
}
