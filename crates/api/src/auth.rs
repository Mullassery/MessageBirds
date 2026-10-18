//! Optional API-key authentication.
//!
//! Set `MESSAGEBIRDS_API_KEYS` (JSON: `{"key": "tenant-uuid"}`) or the
//! single-key `MESSAGEBIRDS_API_KEY` (+ optional `_TENANT`) to require
//! `Authorization: Bearer <key>` on every route except `/healthz`. Keys can be
//! bound to a tenant, and requests with a different `tenant_id` are rejected.
//! This is API-key authentication, not RBAC/SSO; handlers still take
//! `tenant_id` from the request. When no keys are configured, behaviour is
//! unchanged (local dev).

use std::collections::HashMap;

use axum::body::{to_bytes, Body};
use axum::extract::State;
use axum::http::{Request, StatusCode, Uri};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct ApiKeyAuth {
    /// API key -> optional tenant binding.
    keys: HashMap<String, Option<Uuid>>,
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
                    tenant_id.and_then(|value| Uuid::parse_str(value.trim()).ok()),
                );
            }
        }
        Self { keys }
    }

    pub fn from_env() -> Self {
        if let Ok(raw) = std::env::var("MESSAGEBIRDS_API_KEYS") {
            if let Ok(map) = serde_json::from_str::<HashMap<String, String>>(&raw) {
                let keys = map
                    .into_iter()
                    .filter_map(|(key, tenant)| {
                        let key = key.trim().to_string();
                        if key.is_empty() {
                            return None;
                        }
                        let tenant = Uuid::parse_str(tenant.trim()).ok();
                        Some((key, tenant))
                    })
                    .collect();
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

    /// `None` = no keys configured (auth disabled), `Some(tenant)` = valid key
    /// with an optional tenant binding.
    pub fn resolve(&self, header: Option<&str>) -> Option<Option<Uuid>> {
        if !self.enabled() {
            return Some(None);
        }
        let header = header?;
        let provided = header.strip_prefix("Bearer ")?.trim();
        let mut found = None;
        for (key, tenant) in &self.keys {
            if constant_time_eq(key.as_bytes(), provided.as_bytes()) {
                found = Some(*tenant);
            }
        }
        found
    }
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
    let Some(tenant_binding) = auth.resolve(header) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": "missing or invalid API key",
                "hint": "send Authorization: Bearer <key>"
            })),
        )
            .into_response();
    };

    // Per-key tenant binding. This prevents a key from acting across tenants,
    // but it is not RBAC/SSO.
    if let Some(expected) = tenant_binding {
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
        assert_eq!(auth.resolve(None), Some(None));
    }

    #[test]
    fn enabled_auth_requires_the_exact_bearer_token() {
        let auth = ApiKeyAuth::new(Some("secret".to_string()));
        assert_eq!(auth.resolve(Some("Bearer secret")), Some(None));
        assert_eq!(auth.resolve(Some("Bearer wrong")), None);
        assert_eq!(auth.resolve(Some("secret")), None);
        assert_eq!(auth.resolve(None), None);
    }

    #[test]
    fn blank_key_is_treated_as_disabled() {
        let auth = ApiKeyAuth::new(Some("   ".to_string()));
        assert!(!auth.enabled());
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
                ("key-a".to_string(), Some(tenant)),
                ("key-b".to_string(), None),
            ]),
        };
        assert_eq!(auth.resolve(Some("Bearer key-a")), Some(Some(tenant)));
        assert_eq!(auth.resolve(Some("Bearer key-b")), Some(None));
        assert_eq!(auth.resolve(Some("Bearer nope")), None);
    }
}
