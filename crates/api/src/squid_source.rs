//! Squid webhook envelope v1 ingestion.
//!
//! Squid emits outbound record events in a fixed envelope:
//!
//! ```json
//! {
//!   "id": "<uuid>",
//!   "type": "person.created",
//!   "event": "person.created",
//!   "schemaVersion": "1.0",
//!   "workspaceId": "<squid-workspace-uuid>",
//!   "occurredAt": "<rfc3339>",
//!   "data": {}
//! }
//! ```
//!
//! PyCustomerHub's canonical model is OCDS. This module is the explicit
//! mapping layer between them, not a second event model. It verifies the
//! HMAC signature over the exact raw body, resolves the Squid workspace to
//! a PyCustomerHub tenant, and then publishes a normal [`EventEnvelope`]
//! through the same pipeline as every other event.
//!
//! Configuration:
//!
//! ```json
//! {
//!   "<squid-workspace-uuid>": {
//!     "tenant_id": "<pycustomerhub-tenant-uuid>",
//!     "secrets": ["<webhook-secret>", "<rotated-secret>"]
//!   }
//! }
//! ```
//!
//! Set it in `PYCUSTOMERHUB_SQUID_WORKSPACES`. A single `"secret"` string
//! is also accepted. Squid creates a secret per webhook, so a workspace can
//! legitimately have more than one active secret. The source fails closed
//! when a workspace or all of its secrets are missing.
//!
//! Mapping decisions:
//!
//! - `data.id` becomes a `crm_id` identity claim. Person events also carry
//!   the primary email as an `email` claim.
//! - Person name/email/phone fields populate the standard `person` and
//!   `contact` mixins. Other objects remain event data plus context.
//! - `type`/`event` becomes the event type (`person.created`, etc.).
//!   Object-specific schemas (`squid.person@1.0`, ...) are seeded so the
//!   events pass normal schema validation.
//! - Squid does not emit consent metadata. Consent remains a downstream
//!   governance decision; this adapter does not invent one.

use std::collections::HashMap;
use std::env;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use pch_core::{EventEnvelope, EventSource, IdentityRef, MixinRef, SchemaRef, TenantId};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::Sha256;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
struct SquidWorkspace {
    tenant_id: TenantId,
    /// One workspace can have several webhook configurations, each with its
    /// own secret. Accept any secret registered for the workspace.
    secrets: Vec<String>,
}

/// Squid workspace -> (PyCustomerHub tenant, HMAC secrets).
#[derive(Clone, Default)]
pub struct SquidWorkspaces {
    workspaces: HashMap<Uuid, SquidWorkspace>,
}

impl SquidWorkspaces {
    pub fn from_env() -> Self {
        match env::var("PYCUSTOMERHUB_SQUID_WORKSPACES") {
            Ok(raw) => match Self::from_json(&raw) {
                Some(config) => config,
                None => {
                    tracing::warn!(
                        "PYCUSTOMERHUB_SQUID_WORKSPACES must be a JSON object keyed by workspace UUID with tenant_id and secret/secrets"
                    );
                    Self::default()
                }
            },
            Err(_) => Self::default(),
        }
    }

    pub fn from_json(raw: &str) -> Option<Self> {
        let value: Value = serde_json::from_str(raw).ok()?;
        let object = value.as_object()?;
        let mut workspaces = HashMap::new();
        for (workspace_id, config) in object {
            let Ok(workspace_id) = Uuid::parse_str(workspace_id.trim()) else {
                continue;
            };
            let Some(config) = config.as_object() else {
                continue;
            };
            let Some(tenant_id) = config
                .get("tenant_id")
                .and_then(Value::as_str)
                .and_then(|value| Uuid::parse_str(value.trim()).ok())
            else {
                continue;
            };
            let mut secrets = Vec::new();
            if let Some(secret) = config
                .get("secret")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                secrets.push(secret.to_string());
            }
            if let Some(extra) = config.get("secrets").and_then(Value::as_array) {
                secrets.extend(
                    extra
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(str::to_string),
                );
            }
            secrets.sort();
            secrets.dedup();
            if secrets.is_empty() {
                continue;
            }
            workspaces.insert(
                workspace_id,
                SquidWorkspace {
                    tenant_id: TenantId(tenant_id),
                    secrets,
                },
            );
        }
        Some(Self { workspaces })
    }

    pub fn is_configured(&self) -> bool {
        !self.workspaces.is_empty()
    }

    fn get(&self, workspace_id: &Uuid) -> Option<&SquidWorkspace> {
        self.workspaces.get(workspace_id)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SquidEnvelope {
    pub id: Uuid,
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub event: Option<String>,
    #[serde(rename = "schemaVersion", default = "default_schema_version")]
    pub schema_version: String,
    #[serde(rename = "workspaceId")]
    pub workspace_id: String,
    #[serde(rename = "occurredAt")]
    pub occurred_at: String,
    #[serde(default)]
    pub data: Value,
}

fn default_schema_version() -> String {
    "1.0".to_string()
}

impl SquidEnvelope {
    fn operation(&self) -> Result<&str, ApiError> {
        self.kind
            .as_deref()
            .or(self.event.as_deref())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ApiError::BadRequest("webhook envelope has no type/event".to_string()))
    }
}

#[derive(Debug, Serialize)]
pub struct SquidAccepted {
    pub received: u32,
    pub event_id: Uuid,
}

fn verify_with_secret(secret: &str, body: &[u8], signature: &str) -> bool {
    let Some(encoded) = signature.trim().strip_prefix("sha256=") else {
        return false;
    };
    let Ok(signature_bytes) = hex::decode(encoded) else {
        return false;
    };
    let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(body);
    mac.verify_slice(&signature_bytes).is_ok()
}

fn verify_signature(secrets: &[String], body: &[u8], signature: &str) -> bool {
    secrets
        .iter()
        .any(|secret| verify_with_secret(secret, body, signature))
}

fn header_matches(headers: &HeaderMap, name: &str, expected: &str) -> Result<(), ApiError> {
    if let Some(actual) = headers.get(name).and_then(|value| value.to_str().ok()) {
        if actual.trim() != expected {
            return Err(ApiError::BadRequest(format!(
                "header {name} does not match the signed envelope"
            )));
        }
    }
    Ok(())
}

fn string_value(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn person_updates(data: &Map<String, Value>) -> Option<Value> {
    let mut contact = Map::new();
    let mut person = Map::new();

    if let Some(email) = data
        .get("emails")
        .and_then(Value::as_object)
        .and_then(|emails| string_value(emails.get("primaryEmail")))
    {
        contact.insert("email".into(), json!(email));
    }
    if let Some(phone) = data
        .get("phones")
        .and_then(Value::as_object)
        .and_then(|phones| string_value(phones.get("primaryPhoneNumber")))
    {
        contact.insert("phone".into(), json!(phone));
    }
    if let Some(name) = data.get("name").and_then(Value::as_object) {
        if let Some(first) = string_value(name.get("firstName")) {
            person.insert("first_name".into(), json!(first));
        }
        if let Some(last) = string_value(name.get("lastName")) {
            person.insert("last_name".into(), json!(last));
        }
    }

    let mut updates = Map::new();
    if !contact.is_empty() {
        updates.insert(
            MixinRef::standard("contact", "1.0").to_string(),
            Value::Object(contact),
        );
    }
    if !person.is_empty() {
        updates.insert(
            MixinRef::standard("person", "1.0").to_string(),
            Value::Object(person),
        );
    }

    if updates.is_empty() {
        None
    } else {
        Some(Value::Object(updates))
    }
}

fn map_envelope(
    envelope: &SquidEnvelope,
    tenant: TenantId,
    correlation_id: Option<Uuid>,
) -> Result<EventEnvelope, ApiError> {
    if envelope.schema_version != "1.0" {
        return Err(ApiError::BadRequest(format!(
            "unsupported webhook schema version '{}'",
            envelope.schema_version
        )));
    }

    let operation = envelope.operation()?.to_string();
    let (object, action) = operation.split_once('.').ok_or_else(|| {
        ApiError::BadRequest("event type must be '<object>.<action>'".to_string())
    })?;
    if !matches!(
        object,
        "person" | "company" | "opportunity" | "task" | "note"
    ) {
        return Err(ApiError::BadRequest(format!(
            "unsupported Squid object '{object}'"
        )));
    }
    if !matches!(action, "created" | "updated" | "deleted" | "soft_deleted") {
        return Err(ApiError::BadRequest(format!(
            "unsupported Squid action '{action}'"
        )));
    }

    let data = envelope
        .data
        .as_object()
        .ok_or_else(|| ApiError::BadRequest("webhook data must be an object".to_string()))?;
    let record_id = string_value(data.get("id"))
        .ok_or_else(|| ApiError::BadRequest("webhook data.id is required".to_string()))?;

    let mut identity = Vec::new();
    if object == "person" {
        if let Some(email) = data
            .get("emails")
            .and_then(Value::as_object)
            .and_then(|emails| string_value(emails.get("primaryEmail")))
        {
            identity.push(IdentityRef {
                namespace: "email".to_string(),
                value: email,
                primary: false,
                source: "squid-crm".to_string(),
                confidence: 1.0,
            });
        }
    }
    identity.push(IdentityRef {
        namespace: "crm_id".to_string(),
        value: record_id,
        primary: true,
        source: "squid-crm".to_string(),
        confidence: 1.0,
    });

    let timestamp = DateTime::parse_from_rfc3339(&envelope.occurred_at)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| ApiError::BadRequest("occurredAt must be RFC3339".to_string()))?;

    let mut context = Map::new();
    context.insert(
        "squid".into(),
        json!({
            "workspace_id": envelope.workspace_id,
            "schema_version": envelope.schema_version,
            "object": object,
            "operation": operation,
        }),
    );
    if let Some(company_id) = string_value(data.get("companyId")) {
        context.insert("company_id".into(), json!(company_id));
    }
    if let Some(contact_id) = string_value(data.get("pointOfContactId")) {
        context.insert("point_of_contact_id".into(), json!(contact_id));
    }
    if object == "person" {
        if let Some(updates) = person_updates(data) {
            context.insert("profile_updates".into(), updates);
        }
    }

    let schema_name = format!("squid.{object}");

    Ok(EventEnvelope {
        event_id: envelope.id,
        event_type: operation,
        timestamp,
        source: EventSource {
            kind: "squid-crm".to_string(),
            name: "squid-webhook".to_string(),
        },
        identity,
        context: Value::Object(context),
        data: envelope.data.clone(),
        schema: SchemaRef {
            name: schema_name,
            version: "1.0".to_string(),
        },
        consent: None,
        governance: None,
        correlation_id,
        trace_id: None,
        tenant_id: tenant,
    })
}

fn correlation_from_headers(headers: &HeaderMap) -> Option<Uuid> {
    headers
        .get("x-correlation-id")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| Uuid::parse_str(value.trim()).ok())
}

pub async fn webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<SquidAccepted>), ApiError> {
    if !state.squid_workspaces.is_configured() {
        return Err(ApiError::ServiceUnavailable(
            "Squid webhook source is not configured; set PYCUSTOMERHUB_SQUID_WORKSPACES"
                .to_string(),
        ));
    }

    let envelope: SquidEnvelope = serde_json::from_slice(&body)
        .map_err(|error| ApiError::BadRequest(format!("invalid webhook JSON: {error}")))?;

    let workspace_id = Uuid::parse_str(envelope.workspace_id.trim())
        .map_err(|_| ApiError::BadRequest("workspaceId must be a UUID".to_string()))?;
    let workspace = state
        .squid_workspaces
        .get(&workspace_id)
        .ok_or_else(|| ApiError::Unauthorized("unknown Squid workspace".to_string()))?;

    let signature = headers
        .get("x-squid-signature")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| ApiError::Unauthorized("missing x-squid-signature".to_string()))?;
    if !verify_signature(&workspace.secrets, &body, signature) {
        return Err(ApiError::Unauthorized(
            "invalid webhook signature".to_string(),
        ));
    }

    header_matches(&headers, "x-squid-workspace-id", &envelope.workspace_id)?;
    header_matches(&headers, "x-squid-event-id", &envelope.id.to_string())?;
    header_matches(&headers, "x-squid-schema-version", &envelope.schema_version)?;
    let operation = envelope.operation()?;
    header_matches(&headers, "x-squid-event", operation)?;
    header_matches(&headers, "idempotency-key", &envelope.id.to_string())?;

    let event = map_envelope(
        &envelope,
        workspace.tenant_id,
        correlation_from_headers(&headers),
    )?;
    event
        .validate_shape()
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    state
        .producer
        .publish(pch_events::RAW_EVENTS_TOPIC, &event)
        .await?;

    Ok((
        StatusCode::OK,
        Json(SquidAccepted {
            received: 1,
            event_id: event.event_id,
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace_id() -> Uuid {
        Uuid::new_v4()
    }

    fn envelope(data: Value) -> SquidEnvelope {
        serde_json::from_value(json!({
            "id": "8f7a7d5a-23b0-4b9e-9f27-2a4f6f6f0a11",
            "type": "person.created",
            "event": "person.created",
            "schemaVersion": "1.0",
            "workspaceId": workspace_id(),
            "occurredAt": "2026-10-23T10:00:00Z",
            "data": data
        }))
        .unwrap()
    }

    #[test]
    fn workspace_config_requires_tenant_and_secret() {
        let id = workspace_id();
        let tenant = Uuid::new_v4();
        let raw = format!(
            r#"{{"{id}":{{"tenant_id":"{tenant}","secrets":["s3cret","rotated"]}},"bad":{{"tenant_id":"{tenant}","secret":""}}}}"#
        );
        let config = SquidWorkspaces::from_json(&raw).unwrap();
        assert_eq!(config.workspaces.len(), 1);
        let workspace = config.get(&id).unwrap();
        assert_eq!(workspace.tenant_id, TenantId(tenant));
        assert_eq!(workspace.secrets, vec!["rotated", "s3cret"]);
        assert!(config.is_configured());
    }

    #[test]
    fn signature_verification_accepts_any_registered_secret() {
        let secrets = vec!["old".to_string(), "current".to_string()];
        let body = br#"{"id":"x"}"#;
        let mut mac = HmacSha256::new_from_slice(b"current").unwrap();
        mac.update(body);
        let signature = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));
        assert!(verify_signature(&secrets, body, &signature));
        assert!(!verify_signature(&secrets, b"{}", &signature));
        assert!(!verify_signature(&secrets, body, "sha256=deadbeef"));
    }

    #[test]
    fn person_created_maps_identity_and_profile_updates() {
        let envelope = envelope(json!({
            "id": "person-1",
            "name": {"firstName": "Jane", "lastName": "Doe"},
            "emails": {"primaryEmail": "jane@example.com"},
            "phones": {"primaryPhoneNumber": "+15551234567"},
            "companyId": "company-1"
        }));
        let event = map_envelope(&envelope, TenantId(Uuid::nil()), None).unwrap();
        assert_eq!(event.event_type, "person.created");
        assert_eq!(event.schema.name, "squid.person");
        assert_eq!(event.identity.len(), 2);
        let primary = event.identity.iter().find(|claim| claim.primary).unwrap();
        assert_eq!(primary.namespace, "crm_id");
        assert_eq!(primary.value, "person-1");
        assert!(event
            .identity
            .iter()
            .any(|claim| claim.namespace == "email" && claim.value == "jane@example.com"));
        assert_eq!(event.context["company_id"], json!("company-1"));
        let contact = MixinRef::standard("contact", "1.0").to_string();
        let person = MixinRef::standard("person", "1.0").to_string();
        assert_eq!(
            event.context["profile_updates"][&contact]["email"],
            json!("jane@example.com")
        );
        assert_eq!(
            event.context["profile_updates"][&person]["first_name"],
            json!("Jane")
        );
    }

    #[test]
    fn company_event_uses_crm_id_and_does_not_fake_person_updates() {
        let envelope = SquidEnvelope {
            kind: Some("company.updated".into()),
            event: Some("company.updated".into()),
            ..envelope(json!({"id": "company-1", "name": "Acme"}))
        };
        let event = map_envelope(&envelope, TenantId(Uuid::nil()), None).unwrap();
        assert_eq!(event.event_type, "company.updated");
        assert_eq!(event.schema.name, "squid.company");
        assert_eq!(event.identity[0].namespace, "crm_id");
        assert!(event.context.get("profile_updates").is_none());
    }

    #[test]
    fn all_squid_record_operations_map_to_registered_schemas() {
        for object in ["person", "company", "opportunity", "task", "note"] {
            for action in ["created", "updated", "deleted", "soft_deleted"] {
                let operation = format!("{object}.{action}");
                let envelope = SquidEnvelope {
                    kind: Some(operation.clone()),
                    event: Some(operation.clone()),
                    ..envelope(json!({"id": format!("{object}-1")}))
                };
                let event = map_envelope(&envelope, TenantId(Uuid::nil()), None).unwrap();
                assert_eq!(event.event_type, operation);
                assert_eq!(event.schema.name, format!("squid.{object}"));
                assert_eq!(event.schema.version, "1.0");
                assert_eq!(event.identity[0].namespace, "crm_id");
            }
        }
    }

    #[test]
    fn unsupported_operation_is_rejected() {
        let envelope = SquidEnvelope {
            kind: Some("person.archived".into()),
            ..envelope(json!({"id": "person-1"}))
        };
        assert!(map_envelope(&envelope, TenantId(Uuid::nil()), None).is_err());
    }

    #[test]
    fn missing_record_id_is_rejected() {
        let envelope = envelope(json!({"name": {"firstName": "Jane"}}));
        assert!(map_envelope(&envelope, TenantId(Uuid::nil()), None).is_err());
    }

    #[test]
    fn unsupported_schema_version_is_rejected() {
        let envelope = SquidEnvelope {
            schema_version: "2.0".into(),
            ..envelope(json!({"id": "person-1"}))
        };
        assert!(map_envelope(&envelope, TenantId(Uuid::nil()), None).is_err());
    }
}
