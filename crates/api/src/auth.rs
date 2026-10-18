//! Optional API-key authentication.
//!
//! Set `MESSAGEBIRDS_API_KEY` to require `Authorization: Bearer <key>` on every
//! route except `/healthz`. This is a single-key gate, not full multi-tenant
//! isolation: handlers still take `tenant_id` from the request. It closes the
//! "no credential anywhere in the request path" gap without pretending to be
//! RBAC/SSO. When the variable is unset, behaviour is unchanged (local dev).

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::extract::State;
use axum::http::{Request, StatusCode, Uri};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use uuid::Uuid;

#[derive(Clone)]
pub struct ApiKeyAuth {
    key: Option<Arc<str>>,
    tenant_id: Option<Uuid>,
}

impl ApiKeyAuth {
    #[cfg(test)]
    pub fn new(key: Option<String>) -> Self {
        Self::with_tenant(key, None)
    }

    pub fn with_tenant(key: Option<String>, tenant_id: Option<String>) -> Self {
        let key = key
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .map(Arc::from);
        let tenant_id = tenant_id.and_then(|value| Uuid::parse_str(value.trim()).ok());
        Self { key, tenant_id }
    }

    pub fn from_env() -> Self {
        Self::with_tenant(
            std::env::var("MESSAGEBIRDS_API_KEY").ok(),
            std::env::var("MESSAGEBIRDS_API_KEY_TENANT").ok(),
        )
    }

    pub fn tenant_id(&self) -> Option<Uuid> {
        self.tenant_id
    }

    pub fn enabled(&self) -> bool {
        self.key.is_some()
    }

    pub fn is_authorized(&self, header: Option<&str>) -> bool {
        let Some(expected) = self.key.as_deref() else {
            return true;
        };
        let Some(header) = header else {
            return false;
        };
        let Some(provided) = header.strip_prefix("Bearer ") else {
            return false;
        };
        constant_time_eq(expected.as_bytes(), provided.trim().as_bytes())
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
    if auth.enabled() && !auth.is_authorized(header) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": "missing or invalid API key",
                "hint": "send Authorization: Bearer <MESSAGEBIRDS_API_KEY>"
            })),
        )
            .into_response();
    }

    // Single-key tenant binding. This prevents a key from acting across
    // tenants, but it is not RBAC/SSO and does not replace per-tenant keys.
    let expected_tenant = auth.tenant_id();
    if let Some(expected) = expected_tenant {
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
        assert!(auth.is_authorized(None));
    }

    #[test]
    fn enabled_auth_requires_the_exact_bearer_token() {
        let auth = ApiKeyAuth::new(Some("secret".to_string()));
        assert!(auth.is_authorized(Some("Bearer secret")));
        assert!(!auth.is_authorized(Some("Bearer wrong")));
        assert!(!auth.is_authorized(Some("secret")));
        assert!(!auth.is_authorized(None));
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
    fn invalid_tenant_binding_is_ignored() {
        let auth = ApiKeyAuth::with_tenant(Some("secret".into()), Some("not-a-uuid".into()));
        assert_eq!(auth.tenant_id(), None);
    }
}
