//! Optional API-key authentication and coarse roles.
//!
//! Set `MESSAGEBIRDS_API_KEYS` (JSON: `{"key": "tenant-uuid"}` or
//! `{"key": {"tenant_id": "uuid", "role": "read_only"}}`) or the single-key
//! `MESSAGEBIRDS_API_KEY` (+ optional `_TENANT`) to require
//! `Authorization: Bearer <key>` on every route except `/healthz`. Keys can be
//! bound to a tenant, and requests with a different `tenant_id` are rejected.
//! `read_only` keys may only use GET/HEAD/OPTIONS. This is API-key auth plus a
//! coarse write gate, not SSO or a full role system; handlers still take
//! `tenant_id` from the request. When no keys are configured, behaviour is
//! unchanged (local dev).

use std::collections::HashMap;

use axum::body::{to_bytes, Body};
use axum::extract::State;
use axum::http::{Method, Request, StatusCode, Uri};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Admin,
    Writer,
    ReadOnly,
}

#[derive(Clone, Copy, Debug)]
pub struct KeyPolicy {
    pub tenant: Option<Uuid>,
    pub role: Role,
}

#[derive(Clone, Default)]
pub struct ApiKeyAuth {
    keys: HashMap<String, KeyPolicy>,
}

impl ApiKeyAuth {
    #[cfg(test)]
    pub fn new(key: Option<String>) -> Self {
        Self::with_tenant(key, None)
    }

    pub fn with_tenant(key: Option<String>, tenant_id: Option<String>) -> Self {
        let mut keys = HashMap::new();
        if let Some(key) = key.map(|value| value.trim().to_string()) {
            if !key.is_empty() {
                keys.insert(
                    key,
                    KeyPolicy {
                        tenant: tenant_id.and_then(|value| Uuid::parse_str(value.trim()).ok()),
                        role: Role::Admin,
                    },
                );
            }
        }
        Self { keys }
    }

    pub fn from_env() -> Self {
        if let Ok(raw) = std::env::var("MESSAGEBIRDS_API_KEYS") {
            if let Some(keys) = parse_keys(&raw) {
                return Self { keys };
            }
        }
        Self::with_tenant(
            std::env::var("MESSAGEBIRDS_API_KEY").ok(),
            std::env::var("MESSAGEBIRDS_API_KEY_TENANT").ok(),
        )
    }

    pub fn enabled(&self) -> bool {
        !self.keys.is_empty()
    }

    /// `None` = no keys configured (auth disabled), `Some(policy)` = valid key.
    pub fn resolve(&self, header: Option<&str>) -> Option<KeyPolicy> {
        if !self.enabled() {
            return Some(KeyPolicy {
                tenant: None,
                role: Role::Admin,
            });
        }
        let header = header?;
        let provided = header.strip_prefix("Bearer ")?.trim();
        let mut found = None;
        for (key, policy) in &self.keys {
            if constant_time_eq(key.as_bytes(), provided.as_bytes()) {
                found = Some(*policy);
            }
        }
        found
    }
}

fn parse_keys(raw: &str) -> Option<HashMap<String, KeyPolicy>> {
    let value: serde_json::Value = serde_json::from_str(raw).ok()?;
    let object = value.as_object()?;
    let mut keys = HashMap::new();
    for (key, policy) in object {
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        let parsed = match policy {
            serde_json::Value::String(tenant) => KeyPolicy {
                tenant: Uuid::parse_str(tenant.trim()).ok(),
                role: Role::Admin,
            },
            serde_json::Value::Object(map) => {
                let tenant = map
                    .get("tenant_id")
                    .and_then(|value| value.as_str())
                    .and_then(|value| Uuid::parse_str(value.trim()).ok());
                let role = match map.get("role").and_then(|value| value.as_str()) {
                    Some("read_only") | Some("readonly") | Some("viewer") => Role::ReadOnly,
                    Some("writer") | Some("read_write") | Some("operator") => Role::Writer,
                    _ => Role::Admin,
                };
                KeyPolicy { tenant, role }
            }
            _ => continue,
        };
        keys.insert(key.to_string(), parsed);
    }
    Some(keys)
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in left.iter().zip(right) {
        diff |= a ^ b;
    }
    diff == 0
}

pub async fn require(
    State(auth): State<ApiKeyAuth>,
    request: Request<Body>,
    next: Next,
) -> Response {
    if request.uri().path() == "/healthz" {
        return next.run(request).await;
    }
    let header = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());
    let Some(policy) = auth.resolve(header) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": "missing or invalid API key",
                "hint": "send Authorization: Bearer <key>"
            })),
        )
            .into_response();
    };

    if policy.role == Role::ReadOnly && !is_read_method(request.method()) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "this API key is read-only"})),
        )
            .into_response();
    }
    if policy.role == Role::Writer
        && !is_read_method(request.method())
        && is_admin_path(request.uri().path())
    {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "this API key cannot modify configuration resources"})),
        )
            .into_response();
    }

    // Per-key tenant binding. This prevents a key from acting across tenants,
    // but it is not a full role/SSO system.
    if let Some(expected) = policy.tenant {
        let (parts, body) = request.into_parts();
        let bytes = match to_bytes(body, 2 * 1024 * 1024).await {
            Ok(bytes) => bytes,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"error": "request body could not be read"})),
                )
                    .into_response();
            }
        };
        if let Some(requested) = requested_tenant(&parts.uri, &bytes) {
            if requested != expected {
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "error": "tenant_id does not match the authenticated key",
                        "authenticated_tenant": expected.to_string()
                    })),
                )
                    .into_response();
            }
        }
        let request = Request::from_parts(parts, Body::from(bytes));
        return next.run(request).await;
    }

    next.run(request).await
}

fn is_read_method(method: &Method) -> bool {
    matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

fn is_admin_path(path: &str) -> bool {
    const ADMIN_PREFIXES: [&str; 7] = [
        "/destinations",
        "/policies",
        "/channels",
        "/templates",
        "/journeys",
        "/schemas",
        "/mixins",
    ];
    ADMIN_PREFIXES
        .iter()
        .any(|prefix| path == *prefix || path.starts_with(&format!("{prefix}/")))
}

fn requested_tenant(uri: &Uri, body: &[u8]) -> Option<Uuid> {
    if let Some(query) = uri.query() {
        for pair in query.split('&') {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            if key == "tenant_id" {
                return Uuid::parse_str(value).ok();
            }
        }
    }
    let value: serde_json::Value = serde_json::from_slice(body).ok()?;
    value
        .get("tenant_id")
        .and_then(|value| value.as_str())
        .and_then(|value| Uuid::parse_str(value).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_auth_allows_everything() {
        let auth = ApiKeyAuth::new(None);
        let policy = auth.resolve(None).unwrap();
        assert_eq!(policy.role, Role::Admin);
        assert_eq!(policy.tenant, None);
    }

    #[test]
    fn enabled_auth_requires_the_exact_bearer_token() {
        let auth = ApiKeyAuth::new(Some("secret".to_string()));
        assert_eq!(
            auth.resolve(Some("Bearer secret")).unwrap().role,
            Role::Admin
        );
        assert!(auth.resolve(Some("Bearer wrong")).is_none());
        assert!(auth.resolve(Some("secret")).is_none());
        assert!(auth.resolve(None).is_none());
    }

    #[test]
    fn blank_key_is_treated_as_disabled() {
        let auth = ApiKeyAuth::new(Some("   ".to_string()));
        assert!(!auth.enabled());
    }

    #[test]
    fn parses_string_and_object_key_policies() {
        let tenant = Uuid::new_v4();
        let raw = format!(
            r#"{{"admin":"{tenant}","viewer":{{"tenant_id":"{tenant}","role":"read_only"}}}}"#
        );
        let keys = parse_keys(&raw).unwrap();
        assert_eq!(keys["admin"].tenant, Some(tenant));
        assert_eq!(keys["admin"].role, Role::Admin);
        assert_eq!(keys["viewer"].tenant, Some(tenant));
        assert_eq!(keys["viewer"].role, Role::ReadOnly);

        let writer = parse_keys(r#"{"writer":{"role":"writer"}}"#).unwrap();
        assert_eq!(writer["writer"].role, Role::Writer);
    }

    #[test]
    fn read_only_role_is_limited_to_read_methods() {
        assert!(is_read_method(&Method::GET));
        assert!(is_read_method(&Method::HEAD));
        assert!(!is_read_method(&Method::POST));
        assert!(!is_read_method(&Method::DELETE));
    }

    #[test]
    fn writer_is_blocked_from_admin_paths_only() {
        assert!(is_admin_path("/destinations"));
        assert!(is_admin_path("/policies/123"));
        assert!(!is_admin_path("/events"));
        assert!(!is_admin_path("/profiles/1/merge"));
    }

    #[test]
    fn tenant_binding_reads_query_and_body() {
        let tenant = Uuid::new_v4();
        let query: Uri = format!("/profiles?tenant_id={tenant}").parse().unwrap();
        assert_eq!(requested_tenant(&query, b""), Some(tenant));

        let body = format!(r#"{{"tenant_id":"{tenant}","name":"x"}}"#);
        let empty: Uri = "/profiles".parse().unwrap();
        assert_eq!(requested_tenant(&empty, body.as_bytes()), Some(tenant));
    }

    #[test]
    fn maps_multiple_keys_to_tenants() {
        let tenant = Uuid::new_v4();
        let auth = ApiKeyAuth {
            keys: HashMap::from([
                (
                    "key-a".to_string(),
                    KeyPolicy {
                        tenant: Some(tenant),
                        role: Role::Admin,
                    },
                ),
                (
                    "key-b".to_string(),
                    KeyPolicy {
                        tenant: None,
                        role: Role::ReadOnly,
                    },
                ),
            ]),
        };
        assert_eq!(
            auth.resolve(Some("Bearer key-a")).unwrap().tenant,
            Some(tenant)
        );
        assert_eq!(
            auth.resolve(Some("Bearer key-b")).unwrap().role,
            Role::ReadOnly
        );
        assert!(auth.resolve(Some("Bearer nope")).is_none());
    }
}
