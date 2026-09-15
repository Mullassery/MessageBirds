use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

use mb_audiences::AudienceRepo;
use mb_connectors::DestinationRepo;
use mb_governance::{
    consent_purpose_for_action, ConsentEvent, ConsentRepo, ConsentState, Effect, Policy,
    PolicyRepo, Reason,
};
use mb_profile::ProfileRepo;

use crate::error::ApiError;
use crate::labels::{flatten_labels, profile_field_labels};
use crate::routes::audiences::TenantQuery;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreatePolicyRequest {
    pub tenant_id: Uuid,
    pub label: String,
    pub action: String,
    pub effect: Effect,
    #[serde(default)]
    pub priority: i32,
}

pub async fn create_policy(
    State(state): State<AppState>,
    Json(req): Json<CreatePolicyRequest>,
) -> Result<(StatusCode, Json<Policy>), ApiError> {
    let policy = state
        .governance
        .register(
            req.tenant_id,
            &req.label,
            &req.action,
            req.effect,
            req.priority,
        )
        .await?;
    Ok((StatusCode::CREATED, Json(policy)))
}

pub async fn list_policies(
    State(state): State<AppState>,
    Query(q): Query<TenantQuery>,
) -> Result<Json<Vec<Policy>>, ApiError> {
    Ok(Json(state.governance.list(q.tenant_id).await?))
}

#[derive(Deserialize)]
pub struct RecordConsentRequest {
    pub tenant_id: Uuid,
    pub profile_id: Uuid,
    pub purpose: String,
    pub granted: bool,
    pub source: String,
    #[serde(default)]
    pub jurisdiction: Option<String>,
    #[serde(default)]
    pub consent_version: Option<String>,
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
}

pub async fn record_consent(
    State(state): State<AppState>,
    Json(req): Json<RecordConsentRequest>,
) -> Result<(StatusCode, Json<ConsentEvent>), ApiError> {
    let event = state
        .governance
        .record(
            req.tenant_id,
            req.profile_id,
            &req.purpose,
            req.granted,
            &req.source,
            req.jurisdiction,
            req.consent_version,
            req.expires_at,
        )
        .await?;
    Ok((StatusCode::CREATED, Json(event)))
}

#[derive(Serialize)]
pub struct ConsentView {
    pub current: Vec<ConsentState>,
    pub history: Vec<ConsentEvent>,
}

pub async fn get_profile_consent(
    State(state): State<AppState>,
    Query(q): Query<TenantQuery>,
    Path(id): Path<Uuid>,
) -> Result<Json<ConsentView>, ApiError> {
    let current = state.governance.get_all_current(q.tenant_id, id).await?;
    let history = state.governance.get_history(q.tenant_id, id).await?;
    Ok(Json(ConsentView { current, history }))
}

#[derive(Deserialize)]
pub struct PolicySimulateRequest {
    pub tenant_id: Uuid,
    pub audience_id: Uuid,
    pub destination_id: Uuid,
    pub action: String,
}

#[derive(Serialize)]
pub struct ConsentSummary {
    pub granted: usize,
    pub missing: usize,
    pub total: usize,
}

#[derive(Serialize)]
pub struct PolicySimulateResponse {
    pub allowed_fields: Vec<String>,
    pub blocked_fields: Vec<String>,
    pub applicable_policies: Vec<Reason>,
    pub consent_summary: Option<ConsentSummary>,
    pub final_decision: String,
}

/// "Can this audience be activated to this destination for this action?"
/// (Section 49) — built entirely from the same primitives real activation
/// uses (`profile_field_labels`, `evaluate_labels`, `get_current`), not a
/// separate synthetic check, so its answer matches what would actually
/// happen. Label/policy/capability is a data-shape question, checked
/// against every field currently populated across the audience's members;
/// consent is genuinely per-profile, so it's reported as a count rather
/// than folded into a single yes/no.
pub async fn policy_simulate(
    State(state): State<AppState>,
    Json(req): Json<PolicySimulateRequest>,
) -> Result<Json<PolicySimulateResponse>, ApiError> {
    let destination = state
        .connectors
        .get(req.tenant_id, req.destination_id)
        .await?;
    let members = state.audiences.get_members(req.audience_id).await?;
    let purpose = consent_purpose_for_action(&req.action);

    let mut field_labels: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    let mut granted = 0usize;
    let mut missing = 0usize;

    for member in &members {
        if let Some(profile) = state.profiles.get(member.profile_id).await? {
            for (field, labels) in profile_field_labels(&state, &profile).await? {
                field_labels.entry(field).or_default().extend(labels);
            }
        }
        if let Some(purpose) = purpose {
            let has_consent = state
                .governance
                .get_current(req.tenant_id, member.profile_id, purpose)
                .await?
                .map(|s| s.granted)
                .unwrap_or(false);
            if has_consent {
                granted += 1;
            } else {
                missing += 1;
            }
        }
    }

    let all_labels = flatten_labels(&field_labels);
    let mut reasons = state
        .governance
        .evaluate_labels(req.tenant_id, &all_labels, &req.action)
        .await?;

    if !destination
        .supported_actions
        .iter()
        .any(|a| a == &req.action)
    {
        reasons.push(Reason::DestinationCapability {
            action: req.action.clone(),
        });
    }

    let denied_labels: HashSet<String> = reasons
        .iter()
        .filter_map(|r| match r {
            Reason::LabelPolicyDenied { label, .. } => Some(label.clone()),
            _ => None,
        })
        .collect();

    let mut allowed_fields = Vec::new();
    let mut blocked_fields = Vec::new();
    for (field, labels) in &field_labels {
        if labels.iter().any(|l| denied_labels.contains(l)) {
            blocked_fields.push(field.clone());
        } else {
            allowed_fields.push(field.clone());
        }
    }
    allowed_fields.sort();
    blocked_fields.sort();

    let consent_summary = purpose.map(|_| ConsentSummary {
        granted,
        missing,
        total: granted + missing,
    });

    let final_decision = if !reasons.is_empty() {
        "deny"
    } else if consent_summary.as_ref().is_some_and(|c| c.missing > 0) {
        "partial"
    } else {
        "allow"
    };

    Ok(Json(PolicySimulateResponse {
        allowed_fields,
        blocked_fields,
        applicable_policies: reasons,
        consent_summary,
        final_decision: final_decision.to_string(),
    }))
}
