//! Optional API-key authentication.
//!
//! Set `MESSAGEBIRDS_API_KEY` to require `Authorization: Bearer <key>` on every
//! route except `/healthz`. This is a single-key gate, not full multi-tenant
//! isolation: handlers still take `tenant_id` from the request. It closes the
//! "no credential anywhere in the request path" gap without pretending to be
//! RBAC/SSO. When the variable is unset, behaviour is unchanged (local dev).

use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Clone)]
pub struct ApiKeyAuth {
    key: Option<Arc<str>>,
}

impl ApiKeyAuth {
    pub fn new(key: Option<String>) -> Self {
        let key = key
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .map(Arc::from);
        Self { key }
    }

    pub fn from_env() -> Self {
        Self::new(std::env::var("MESSAGEBIRDS_API_KEY").ok())
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
    if !auth.enabled() || auth.is_authorized(header) {
        next.run(request).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": "missing or invalid API key",
                "hint": "send Authorization: Bearer <MESSAGEBIRDS_API_KEY>"
            })),
        )
            .into_response()
    }
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
}
