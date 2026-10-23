//! HTTP tracking-source compatibility.
//!
//! PyCustomerHub's native ingestion contract is its OCDS envelope
//! (`POST /events`, see `docs/OCDS.md`). This module adds a second,
//! lower-friction front door: the common `track` / `identify` / `page` /
//! `screen` / `group` / `alias` HTTP shape used by analytics SDKs, plus a
//! batch endpoint. Existing clients can point at this host and keep their
//! instrumentation.
//!
//! This is a translation layer, not a second event model. Every accepted
//! message is converted to the canonical [`EventEnvelope`] and then goes
//! through the same schema validation, identity resolution, merge policy,
//! audiences, and journeys as `POST /events`.
//!
//! Deliberate boundaries:
//!
//! - `group` calls update the acting user's profile and preserve the group
//!   id in event context. Group ids are *not* identity claims: the profile
//!   model is person-centric, and merging every member of an account into
//!   one profile would be wrong.
//! - Only `identify` traits feed `profile_updates`; arbitrary `track`
//!   properties are event data, not profile mutation.
//! - Unknown client context fields are preserved verbatim under
//!   `context.tracking_context`, never dropped.
//! - Write keys are required. A deployment must map at least one key to a
//!   tenant via `PYCUSTOMERHUB_HTTP_WRITE_KEYS`; there is no unauthenticated
//!   fallback.

use std::collections::HashMap;
use std::env;

use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::Json;
use base64::Engine;
use chrono::{DateTime, Utc};
use pch_core::{EventEnvelope, EventSource, IdentityRef, MixinRef, SchemaRef, TenantId};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;

const MAX_BATCH_SIZE: usize = 1_000;

/// Write-key -> tenant mapping for the HTTP tracking source.
///
/// Accepted values are tenant UUIDs:
/// `{"key-a":"<tenant-uuid>","key-b":"<tenant-uuid>"}`.
#[derive(Clone, Default)]
pub struct HttpWriteKeys {
    keys: HashMap<String, TenantId>,
}

impl HttpWriteKeys {
    pub fn from_env() -> Self {
        match env::var("PYCUSTOMERHUB_HTTP_WRITE_KEYS") {
            Ok(raw) => match Self::from_json(&raw) {
                Some(keys) => keys,
                None => {
                    tracing::warn!(
                        "PYCUSTOMERHUB_HTTP_WRITE_KEYS is not a JSON object of write-key -> tenant UUID"
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
        let mut keys = HashMap::new();
        for (key, tenant) in object {
            let key = key.trim();
            if key.is_empty() {
                continue;
            }
            let Some(tenant) = tenant
                .as_str()
                .and_then(|value| Uuid::parse_str(value.trim()).ok())
            else {
                continue;
            };
            keys.insert(key.to_string(), TenantId(tenant));
        }
        Some(Self { keys })
    }

    pub fn is_configured(&self) -> bool {
        !self.keys.is_empty()
    }

    fn tenant_for(&self, key: &str) -> Option<TenantId> {
        self.keys.get(key).copied()
    }
}

/// Supported call shapes on the compatibility endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackingCall {
    Track,
    Identify,
    Page,
    Screen,
    Group,
    Alias,
}

impl TrackingCall {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Track => "track",
            Self::Identify => "identify",
            Self::Page => "page",
            Self::Screen => "screen",
            Self::Group => "group",
            Self::Alias => "alias",
        }
    }

    fn default_event_type(&self) -> &'static str {
        match self {
            Self::Track => "http.track",
            Self::Identify => "http.identify",
            Self::Page => "http.page",
            Self::Screen => "http.screen",
            Self::Group => "http.group",
            Self::Alias => "http.alias",
        }
    }

    fn schema_name(&self) -> &'static str {
        match self {
            Self::Track => "http.track",
            Self::Identify => "http.identify",
            Self::Page => "http.page",
            Self::Screen => "http.screen",
            Self::Group => "http.group",
            Self::Alias => "http.alias",
        }
    }
}

/// One tracking message, loose by design because client context is open-ended.
#[derive(Debug, Clone, Deserialize)]
pub struct TrackingMessage {
    #[serde(rename = "type", default)]
    pub call: Option<String>,
    #[serde(default)]
    pub event: Option<String>,
    #[serde(default, rename = "userId")]
    pub user_id: Option<Value>,
    #[serde(default, rename = "anonymousId")]
    pub anonymous_id: Option<Value>,
    #[serde(default, rename = "groupId")]
    pub group_id: Option<Value>,
    #[serde(default, rename = "previousId")]
    pub previous_id: Option<Value>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub properties: Option<Value>,
    #[serde(default)]
    pub traits: Option<Value>,
    #[serde(default)]
    pub context: Option<Value>,
    #[serde(default)]
    pub integrations: Option<Value>,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default, rename = "messageId")]
    pub message_id: Option<String>,
    #[serde(default, rename = "writeKey")]
    pub write_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BatchPayload {
    #[serde(default)]
    pub batch: Vec<TrackingMessage>,
    #[serde(default, rename = "writeKey")]
    pub write_key: Option<String>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TrackingError {
    #[error("unknown call type '{0}'")]
    UnknownCall(String),
    #[error("{0} messages require an event name")]
    MissingEvent(&'static str),
    #[error("message has no userId or anonymousId to identify the profile")]
    NoIdentity,
    #[error("alias messages require both userId and previousId")]
    IncompleteAlias,
}

#[derive(Debug, Serialize)]
pub struct Accepted {
    pub received: usize,
}

fn as_string(value: &Option<Value>) -> Option<String> {
    match value {
        Some(Value::String(s)) if !s.trim().is_empty() => Some(s.clone()),
        Some(Value::Number(n)) => Some(n.to_string()),
        _ => None,
    }
}

fn trait_string(msg: &TrackingMessage, key: &str) -> Option<String> {
    msg.traits
        .as_ref()
        .and_then(Value::as_object)
        .and_then(|traits| traits.get(key))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn identity_claims(msg: &TrackingMessage, call: TrackingCall) -> Vec<IdentityRef> {
    let mut claims = Vec::new();
    let mut push = |namespace: &str, value: Option<String>| {
        if let Some(value) = value {
            claims.push(IdentityRef {
                namespace: namespace.to_string(),
                value,
                primary: false,
                source: "http-tracking".to_string(),
                confidence: 1.0,
            });
        }
    };

    push("anonymous_id", as_string(&msg.anonymous_id));
    if call == TrackingCall::Alias {
        push("customer_id", as_string(&msg.previous_id));
    }
    if call == TrackingCall::Identify {
        push("email", trait_string(msg, "email"));
    }
    // Highest-priority last: the final claim becomes primary.
    push("customer_id", as_string(&msg.user_id));

    if let Some(claim) = claims.last_mut() {
        claim.primary = true;
    }
    claims
}

/// Maps `identify` traits to the standard profile mixins. Unknown traits
/// remain in the event payload.
fn profile_updates(msg: &TrackingMessage, call: TrackingCall) -> Option<Value> {
    if call != TrackingCall::Identify {
        return None;
    }
    let traits = msg.traits.as_ref().and_then(Value::as_object)?;

    let mut contact = Map::new();
    let mut person = Map::new();

    if let Some(email) = traits.get("email").filter(|v| v.is_string()) {
        contact.insert("email".into(), email.clone());
    }
    if let Some(phone) = traits.get("phone").filter(|v| v.is_string()) {
        contact.insert("phone".into(), phone.clone());
    }
    if let Some(address) = traits.get("address").filter(|v| v.is_object()) {
        contact.insert("address".into(), address.clone());
    }
    if let Some(first) = traits
        .get("firstName")
        .or_else(|| traits.get("first_name"))
        .filter(|v| v.is_string())
    {
        person.insert("first_name".into(), first.clone());
    }
    if let Some(last) = traits
        .get("lastName")
        .or_else(|| traits.get("last_name"))
        .filter(|v| v.is_string())
    {
        person.insert("last_name".into(), last.clone());
    }
    if let Some(birth) = traits.get("birthday").filter(|v| v.is_string()) {
        person.insert("birth_date".into(), birth.clone());
    }
    if let Some(gender) = traits.get("gender").filter(|v| v.is_string()) {
        if matches!(
            gender.as_str(),
            Some("male" | "female" | "other" | "unspecified")
        ) {
            person.insert("gender".into(), gender.clone());
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

fn parse_timestamp(raw: &Option<String>) -> DateTime<Utc> {
    if let Some(raw) = raw {
        let raw = raw.trim();
        if let Ok(parsed) = DateTime::parse_from_rfc3339(raw) {
            return parsed.with_timezone(&Utc);
        }
        if let Ok(millis) = raw.parse::<i64>() {
            let seconds = millis.div_euclid(1_000);
            let nanos = (millis.rem_euclid(1_000) * 1_000_000) as u32;
            if let Some(parsed) = DateTime::from_timestamp(seconds, nanos) {
                return parsed;
            }
        }
    }
    Utc::now()
}

/// Converts one message into the canonical envelope. The tenant is already
/// authenticated by the caller; `writeKey` in the payload is never allowed
/// to switch tenants.
pub fn translate(
    call: TrackingCall,
    msg: &TrackingMessage,
    tenant: TenantId,
) -> Result<EventEnvelope, TrackingError> {
    let event_type = msg
        .event
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| call.default_event_type().to_string());

    if call == TrackingCall::Track
        && msg
            .event
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
    {
        return Err(TrackingError::MissingEvent("track"));
    }
    if call == TrackingCall::Alias
        && (as_string(&msg.user_id).is_none() || as_string(&msg.previous_id).is_none())
    {
        return Err(TrackingError::IncompleteAlias);
    }

    let identity = identity_claims(msg, call);
    if identity.is_empty() {
        return Err(TrackingError::NoIdentity);
    }

    let mut context = Map::new();
    context.insert("tracking_call".into(), json!(call.as_str()));
    if let Some(name) = &msg.name {
        context.insert("name".into(), json!(name));
    }
    if let Some(category) = &msg.category {
        context.insert("category".into(), json!(category));
    }
    if let Some(group_id) = as_string(&msg.group_id) {
        context.insert("group_id".into(), json!(group_id));
    }
    if let Some(tracking_context) = &msg.context {
        context.insert("tracking_context".into(), tracking_context.clone());
    }
    if let Some(integrations) = &msg.integrations {
        context.insert("integrations".into(), integrations.clone());
    }
    if let Some(updates) = profile_updates(msg, call) {
        context.insert("profile_updates".into(), updates);
    }

    let mut data = Map::new();
    data.insert("tracking_call".into(), json!(call.as_str()));
    if let Some(properties) = &msg.properties {
        data.insert("properties".into(), properties.clone());
    }
    if let Some(traits) = &msg.traits {
        data.insert("traits".into(), traits.clone());
    }
    if let Some(event_name) = msg
        .event
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        data.insert("event_name".into(), json!(event_name));
    }

    Ok(EventEnvelope {
        event_id: msg
            .message_id
            .as_deref()
            .and_then(|id| Uuid::parse_str(id.trim()).ok())
            .unwrap_or_else(Uuid::new_v4),
        event_type,
        timestamp: parse_timestamp(&msg.timestamp),
        source: EventSource {
            kind: "http-tracking".to_string(),
            name: "http-tracking".to_string(),
        },
        identity,
        context: Value::Object(context),
        data: Value::Object(data),
        schema: SchemaRef {
            name: call.schema_name().to_string(),
            version: "1.0".to_string(),
        },
        consent: None,
        governance: None,
        correlation_id: None,
        trace_id: msg.message_id.clone(),
        tenant_id: tenant,
    })
}

pub fn call_from_str(raw: &str) -> Result<TrackingCall, TrackingError> {
    match raw {
        "track" => Ok(TrackingCall::Track),
        "identify" => Ok(TrackingCall::Identify),
        "page" => Ok(TrackingCall::Page),
        "screen" => Ok(TrackingCall::Screen),
        "group" => Ok(TrackingCall::Group),
        "alias" => Ok(TrackingCall::Alias),
        other => Err(TrackingError::UnknownCall(other.to_string())),
    }
}

fn resolve_tenant(
    keys: &HttpWriteKeys,
    headers: &HeaderMap,
    body_key: Option<&str>,
) -> Result<TenantId, ApiError> {
    if !keys.is_configured() {
        return Err(ApiError::ServiceUnavailable(
            "HTTP tracking source is not configured; set PYCUSTOMERHUB_HTTP_WRITE_KEYS".to_string(),
        ));
    }

    let header_key = header_write_key(headers);
    let body_key = body_key
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    if let (Some(header_key), Some(body_key)) = (&header_key, &body_key) {
        if header_key != body_key {
            return Err(ApiError::Unauthorized(
                "write key in the request body does not match the authorization header".to_string(),
            ));
        }
    }

    let key = body_key
        .or(header_key)
        .ok_or_else(|| ApiError::Unauthorized("missing write key".to_string()))?;

    keys.tenant_for(&key)
        .ok_or_else(|| ApiError::Unauthorized("unknown write key".to_string()))
}

fn header_write_key(headers: &HeaderMap) -> Option<String> {
    if let Some(value) = headers
        .get("x-write-key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Some(value.to_string());
    }

    let authorization = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())?;
    let (scheme, encoded) = authorization.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("basic") {
        return None;
    }
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded.trim())
        .ok()?;
    let text = String::from_utf8(decoded).ok()?;
    let key = text.split(':').next()?.trim();
    if key.is_empty() {
        None
    } else {
        Some(key.to_string())
    }
}

async fn ingest(
    state: &AppState,
    headers: &HeaderMap,
    call: TrackingCall,
    msg: &TrackingMessage,
) -> Result<EventEnvelope, ApiError> {
    let tenant = resolve_tenant(&state.http_write_keys, headers, msg.write_key.as_deref())?;
    let envelope =
        translate(call, msg, tenant).map_err(|error| ApiError::BadRequest(error.to_string()))?;
    envelope
        .validate_shape()
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    state
        .producer
        .publish(pch_events::RAW_EVENTS_TOPIC, &envelope)
        .await?;
    Ok(envelope)
}

async fn ingest_one(
    state: AppState,
    headers: HeaderMap,
    call: TrackingCall,
    msg: TrackingMessage,
) -> Result<(StatusCode, Json<Accepted>), ApiError> {
    ingest(&state, &headers, call, &msg).await?;
    Ok((StatusCode::OK, Json(Accepted { received: 1 })))
}

pub async fn track(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(msg): Json<TrackingMessage>,
) -> Result<(StatusCode, Json<Accepted>), ApiError> {
    ingest_one(state, headers, TrackingCall::Track, msg).await
}

pub async fn identify(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(msg): Json<TrackingMessage>,
) -> Result<(StatusCode, Json<Accepted>), ApiError> {
    ingest_one(state, headers, TrackingCall::Identify, msg).await
}

pub async fn page(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(msg): Json<TrackingMessage>,
) -> Result<(StatusCode, Json<Accepted>), ApiError> {
    ingest_one(state, headers, TrackingCall::Page, msg).await
}

pub async fn screen(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(msg): Json<TrackingMessage>,
) -> Result<(StatusCode, Json<Accepted>), ApiError> {
    ingest_one(state, headers, TrackingCall::Screen, msg).await
}

pub async fn group(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(msg): Json<TrackingMessage>,
) -> Result<(StatusCode, Json<Accepted>), ApiError> {
    ingest_one(state, headers, TrackingCall::Group, msg).await
}

pub async fn alias(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(msg): Json<TrackingMessage>,
) -> Result<(StatusCode, Json<Accepted>), ApiError> {
    ingest_one(state, headers, TrackingCall::Alias, msg).await
}

pub async fn batch(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<BatchPayload>,
) -> Result<(StatusCode, Json<Accepted>), ApiError> {
    if payload.batch.is_empty() {
        return Err(ApiError::BadRequest("batch must not be empty".to_string()));
    }
    if payload.batch.len() > MAX_BATCH_SIZE {
        return Err(ApiError::BadRequest(format!(
            "batch exceeds the {MAX_BATCH_SIZE}-message limit"
        )));
    }

    let body_key = payload
        .write_key
        .as_deref()
        .or_else(|| payload.batch.first().and_then(|m| m.write_key.as_deref()));
    let tenant = resolve_tenant(&state.http_write_keys, &headers, body_key)?;

    let mut envelopes = Vec::with_capacity(payload.batch.len());
    for (index, msg) in payload.batch.iter().enumerate() {
        let call = call_from_str(
            msg.call
                .as_deref()
                .ok_or_else(|| TrackingError::UnknownCall(String::new()))
                .map_err(|error| ApiError::BadRequest(format!("batch[{index}]: {error}")))?,
        )
        .map_err(|error| ApiError::BadRequest(format!("batch[{index}]: {error}")))?;
        let envelope = translate(call, msg, tenant)
            .map_err(|error| ApiError::BadRequest(format!("batch[{index}]: {error}")))?;
        envelope
            .validate_shape()
            .map_err(|error| ApiError::BadRequest(format!("batch[{index}]: {error}")))?;
        envelopes.push(envelope);
    }

    for envelope in &envelopes {
        state
            .producer
            .publish(pch_events::RAW_EVENTS_TOPIC, envelope)
            .await?;
    }

    Ok((
        StatusCode::OK,
        Json(Accepted {
            received: envelopes.len(),
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tenant() -> TenantId {
        TenantId(Uuid::nil())
    }

    fn msg(value: Value) -> TrackingMessage {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn track_maps_event_properties_and_anonymous_id() {
        let message = msg(json!({
            "anonymousId": "anon-1",
            "event": "Order Completed",
            "properties": {"revenue": 42}
        }));
        let envelope = translate(TrackingCall::Track, &message, tenant()).unwrap();
        assert_eq!(envelope.event_type, "Order Completed");
        assert_eq!(envelope.schema.name, "http.track");
        assert_eq!(envelope.identity.len(), 1);
        assert_eq!(envelope.identity[0].namespace, "anonymous_id");
        assert!(envelope.identity[0].primary);
        assert_eq!(envelope.data["properties"]["revenue"], json!(42));
        assert_eq!(envelope.source.kind, "http-tracking");
    }

    #[test]
    fn identify_maps_traits_into_profile_updates() {
        let message = msg(json!({
            "userId": "user-9",
            "traits": {
                "email": "jane@example.com",
                "firstName": "Jane",
                "lastName": "Doe"
            }
        }));
        let envelope = translate(TrackingCall::Identify, &message, tenant()).unwrap();
        assert_eq!(envelope.event_type, "http.identify");

        let contact = MixinRef::standard("contact", "1.0").to_string();
        let person = MixinRef::standard("person", "1.0").to_string();
        let updates = &envelope.context["profile_updates"];
        assert_eq!(updates[&contact]["email"], json!("jane@example.com"));
        assert_eq!(updates[&person]["first_name"], json!("Jane"));
        assert_eq!(updates[&person]["last_name"], json!("Doe"));

        let primary = envelope
            .identity
            .iter()
            .find(|claim| claim.primary)
            .unwrap();
        assert_eq!(primary.namespace, "customer_id");
        assert!(envelope
            .identity
            .iter()
            .any(|claim| claim.namespace == "email" && claim.value == "jane@example.com"));
    }

    #[test]
    fn track_without_event_name_is_rejected() {
        let message = msg(json!({"anonymousId": "a"}));
        assert_eq!(
            translate(TrackingCall::Track, &message, tenant()).unwrap_err(),
            TrackingError::MissingEvent("track")
        );
    }

    #[test]
    fn message_without_person_identity_is_rejected() {
        let message = msg(json!({"event": "x", "groupId": "acct-1"}));
        assert_eq!(
            translate(TrackingCall::Track, &message, tenant()).unwrap_err(),
            TrackingError::NoIdentity
        );
    }

    #[test]
    fn group_id_is_context_not_a_profile_identity() {
        let message = msg(json!({
            "anonymousId": "anon-2",
            "groupId": "acct-1",
            "traits": {"name": "Acme"}
        }));
        let envelope = translate(TrackingCall::Group, &message, tenant()).unwrap();
        assert_eq!(envelope.context["group_id"], json!("acct-1"));
        assert!(envelope
            .identity
            .iter()
            .all(|claim| claim.namespace != "group_id"));
    }

    #[test]
    fn page_defaults_event_type_and_preserves_client_context() {
        let message = msg(json!({
            "anonymousId": "anon-3",
            "name": "Home",
            "context": {"userAgent": "curl"}
        }));
        let envelope = translate(TrackingCall::Page, &message, tenant()).unwrap();
        assert_eq!(envelope.event_type, "http.page");
        assert_eq!(envelope.context["name"], json!("Home"));
        assert_eq!(
            envelope.context["tracking_context"]["userAgent"],
            json!("curl")
        );
    }

    #[test]
    fn alias_requires_both_ids() {
        let message = msg(json!({"userId": "new-id"}));
        assert_eq!(
            translate(TrackingCall::Alias, &message, tenant()).unwrap_err(),
            TrackingError::IncompleteAlias
        );
    }

    #[test]
    fn alias_links_previous_and_current_ids() {
        let message = msg(json!({"userId": "new-id", "previousId": "old-id"}));
        let envelope = translate(TrackingCall::Alias, &message, tenant()).unwrap();
        let ids: Vec<_> = envelope
            .identity
            .iter()
            .map(|claim| {
                (
                    claim.namespace.as_str(),
                    claim.value.as_str(),
                    claim.primary,
                )
            })
            .collect();
        assert_eq!(
            ids,
            vec![
                ("customer_id", "old-id", false),
                ("customer_id", "new-id", true)
            ]
        );
    }

    #[test]
    fn message_write_key_cannot_switch_tenant() {
        let message = msg(json!({
            "anonymousId": "a",
            "event": "x",
            "writeKey": Uuid::new_v4().to_string()
        }));
        let envelope = translate(TrackingCall::Track, &message, tenant()).unwrap();
        assert_eq!(envelope.tenant_id, tenant());
    }

    #[test]
    fn batch_translates_every_element() {
        let payload: BatchPayload = serde_json::from_value(json!({
            "batch": [
                {"type": "identify", "userId": "u1", "traits": {"email": "a@b.c"}},
                {"type": "track", "anonymousId": "a1", "event": "Clicked"}
            ]
        }))
        .unwrap();
        let events: Vec<_> = payload
            .batch
            .iter()
            .map(|msg| {
                translate(
                    call_from_str(msg.call.as_deref().unwrap()).unwrap(),
                    msg,
                    tenant(),
                )
                .unwrap()
            })
            .collect();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, "http.identify");
        assert_eq!(events[1].event_type, "Clicked");
    }

    #[test]
    fn write_keys_parse_only_uuid_tenants() {
        let id = Uuid::new_v4();
        let keys = HttpWriteKeys::from_json(&format!(
            r#"{{"good":"{id}","bad":"not-a-uuid","empty":""}}"#
        ))
        .unwrap();
        assert_eq!(keys.tenant_for("good"), Some(TenantId(id)));
        assert_eq!(keys.tenant_for("bad"), None);
        assert_eq!(keys.tenant_for("empty"), None);
        assert!(keys.is_configured());
    }

    #[test]
    fn basic_auth_extracts_write_key() {
        let mut headers = HeaderMap::new();
        let encoded = base64::engine::general_purpose::STANDARD.encode("write-key:");
        headers.insert(
            header::AUTHORIZATION,
            format!("Basic {encoded}").parse().unwrap(),
        );
        assert_eq!(header_write_key(&headers), Some("write-key".to_string()));
    }

    #[test]
    fn mismatched_body_and_header_keys_are_rejected() {
        let id = Uuid::new_v4();
        let keys = HttpWriteKeys::from_json(&format!(r#"{{"header-key":"{id}"}}"#)).unwrap();
        let mut headers = HeaderMap::new();
        headers.insert("x-write-key", "header-key".parse().unwrap());
        assert!(matches!(
            resolve_tenant(&keys, &headers, Some("body-key")),
            Err(ApiError::Unauthorized(_))
        ));
    }

    #[test]
    fn unconfigured_source_fails_closed() {
        let keys = HttpWriteKeys::default();
        assert!(matches!(
            resolve_tenant(&keys, &HeaderMap::new(), Some("key")),
            Err(ApiError::ServiceUnavailable(_))
        ));
    }
}
