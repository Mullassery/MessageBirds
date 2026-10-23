use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    Unauthorized(String),
    NotFound(String),
    Conflict(String),
    Forbidden(String),
    ServiceUnavailable(String),
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::BadRequest(m) => (StatusCode::BAD_REQUEST, m),
            ApiError::Unauthorized(m) => (StatusCode::UNAUTHORIZED, m),
            ApiError::NotFound(m) => (StatusCode::NOT_FOUND, m),
            ApiError::Conflict(m) => (StatusCode::CONFLICT, m),
            ApiError::Forbidden(m) => (StatusCode::FORBIDDEN, m),
            ApiError::ServiceUnavailable(m) => (StatusCode::SERVICE_UNAVAILABLE, m),
            ApiError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, m),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

impl From<pch_schema_registry::SchemaError> for ApiError {
    fn from(e: pch_schema_registry::SchemaError) -> Self {
        use pch_schema_registry::SchemaError::*;
        match e {
            AlreadyExists { .. } | IncompatibleChange { .. } => ApiError::Conflict(e.to_string()),
            NotFound { .. } | NoVersions(_) => ApiError::NotFound(e.to_string()),
            Corrupt(_) | Db(_) => ApiError::Internal(e.to_string()),
        }
    }
}

impl From<pch_mixins::MixinError> for ApiError {
    fn from(e: pch_mixins::MixinError) -> Self {
        use pch_mixins::MixinError::*;
        match e {
            ReservedNamespace => ApiError::Forbidden(e.to_string()),
            AlreadyExists { .. } => ApiError::Conflict(e.to_string()),
            NotFound { .. } | NoVersions { .. } => ApiError::NotFound(e.to_string()),
            Corrupt(_) | Db(_) => ApiError::Internal(e.to_string()),
        }
    }
}

impl From<pch_namespaces::NamespaceError> for ApiError {
    fn from(e: pch_namespaces::NamespaceError) -> Self {
        use pch_namespaces::NamespaceError::*;
        match e {
            AlreadyExists(_) => ApiError::Conflict(e.to_string()),
            NotFound(_) => ApiError::NotFound(e.to_string()),
            Db(_) => ApiError::Internal(e.to_string()),
        }
    }
}

impl From<pch_profile::ProfileError> for ApiError {
    fn from(e: pch_profile::ProfileError) -> Self {
        use pch_profile::ProfileError::*;
        match e {
            NotAnObject => ApiError::BadRequest(e.to_string()),
            MergePolicy(_) | Db(_) => ApiError::Internal(e.to_string()),
        }
    }
}

impl From<pch_identity::IdentityError> for ApiError {
    fn from(e: pch_identity::IdentityError) -> Self {
        use pch_identity::IdentityError::*;
        match e {
            NotLinked { .. } => ApiError::BadRequest(e.to_string()),
            Db(_) => ApiError::Internal(e.to_string()),
        }
    }
}

impl From<pch_audiences::AudienceError> for ApiError {
    fn from(e: pch_audiences::AudienceError) -> Self {
        use pch_audiences::AudienceError::*;
        match e {
            NotFound(_) => ApiError::NotFound(e.to_string()),
            Corrupt(_) | Db(_) => ApiError::Internal(e.to_string()),
        }
    }
}

impl From<pch_connectors::ConnectorRepoError> for ApiError {
    fn from(e: pch_connectors::ConnectorRepoError) -> Self {
        use pch_connectors::ConnectorRepoError::*;
        match e {
            NotFound(_) => ApiError::NotFound(e.to_string()),
            Db(_) => ApiError::Internal(e.to_string()),
        }
    }
}

impl From<pch_governance::GovernanceError> for ApiError {
    fn from(e: pch_governance::GovernanceError) -> Self {
        use pch_governance::GovernanceError::*;
        match e {
            Db(_) => ApiError::Internal(e.to_string()),
        }
    }
}

impl From<pch_channels::ChannelRepoError> for ApiError {
    fn from(e: pch_channels::ChannelRepoError) -> Self {
        use pch_channels::ChannelRepoError::*;
        match e {
            NotFound(_) => ApiError::NotFound(e.to_string()),
            Db(_) => ApiError::Internal(e.to_string()),
        }
    }
}

impl From<pch_templates::TemplateError> for ApiError {
    fn from(e: pch_templates::TemplateError) -> Self {
        use pch_templates::TemplateError::*;
        match e {
            NotFound(_) | NoVersions(_) => ApiError::NotFound(e.to_string()),
            Db(_) => ApiError::Internal(e.to_string()),
        }
    }
}

impl From<pch_journeys::JourneyError> for ApiError {
    fn from(e: pch_journeys::JourneyError) -> Self {
        use pch_journeys::JourneyError::*;
        match e {
            NotFound(_) | RunNotFound(_) | ProfileNotFound(_) => ApiError::NotFound(e.to_string()),
            UnknownNode(_) => ApiError::BadRequest(e.to_string()),
            Corrupt(_) | Audience(_) | Profile(_) | Template(_) | Channel(_) | Db(_) => {
                ApiError::Internal(e.to_string())
            }
        }
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        ApiError::Internal(e.to_string())
    }
}

impl From<pch_events::EventsError> for ApiError {
    fn from(e: pch_events::EventsError) -> Self {
        ApiError::Internal(e.to_string())
    }
}
