use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

pub enum ApiError {
    BadRequest(String),
    NotFound(String),
    Conflict(String),
    Forbidden(String),
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::BadRequest(m) => (StatusCode::BAD_REQUEST, m),
            ApiError::NotFound(m) => (StatusCode::NOT_FOUND, m),
            ApiError::Conflict(m) => (StatusCode::CONFLICT, m),
            ApiError::Forbidden(m) => (StatusCode::FORBIDDEN, m),
            ApiError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, m),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

impl From<mb_schema_registry::SchemaError> for ApiError {
    fn from(e: mb_schema_registry::SchemaError) -> Self {
        use mb_schema_registry::SchemaError::*;
        match e {
            AlreadyExists { .. } | IncompatibleChange { .. } => ApiError::Conflict(e.to_string()),
            NotFound { .. } | NoVersions(_) => ApiError::NotFound(e.to_string()),
            Corrupt(_) | Db(_) => ApiError::Internal(e.to_string()),
        }
    }
}

impl From<mb_mixins::MixinError> for ApiError {
    fn from(e: mb_mixins::MixinError) -> Self {
        use mb_mixins::MixinError::*;
        match e {
            ReservedNamespace => ApiError::Forbidden(e.to_string()),
            AlreadyExists { .. } => ApiError::Conflict(e.to_string()),
            NotFound { .. } | NoVersions { .. } => ApiError::NotFound(e.to_string()),
            Corrupt(_) | Db(_) => ApiError::Internal(e.to_string()),
        }
    }
}

impl From<mb_namespaces::NamespaceError> for ApiError {
    fn from(e: mb_namespaces::NamespaceError) -> Self {
        use mb_namespaces::NamespaceError::*;
        match e {
            AlreadyExists(_) => ApiError::Conflict(e.to_string()),
            NotFound(_) => ApiError::NotFound(e.to_string()),
            Db(_) => ApiError::Internal(e.to_string()),
        }
    }
}

impl From<mb_profile::ProfileError> for ApiError {
    fn from(e: mb_profile::ProfileError) -> Self {
        use mb_profile::ProfileError::*;
        match e {
            NotAnObject => ApiError::BadRequest(e.to_string()),
            MergePolicy(_) | Db(_) => ApiError::Internal(e.to_string()),
        }
    }
}

impl From<mb_identity::IdentityError> for ApiError {
    fn from(e: mb_identity::IdentityError) -> Self {
        use mb_identity::IdentityError::*;
        match e {
            NotLinked { .. } => ApiError::BadRequest(e.to_string()),
            Db(_) => ApiError::Internal(e.to_string()),
        }
    }
}

impl From<mb_audiences::AudienceError> for ApiError {
    fn from(e: mb_audiences::AudienceError) -> Self {
        use mb_audiences::AudienceError::*;
        match e {
            NotFound(_) => ApiError::NotFound(e.to_string()),
            Corrupt(_) | Db(_) => ApiError::Internal(e.to_string()),
        }
    }
}

impl From<mb_connectors::ConnectorRepoError> for ApiError {
    fn from(e: mb_connectors::ConnectorRepoError) -> Self {
        use mb_connectors::ConnectorRepoError::*;
        match e {
            NotFound(_) => ApiError::NotFound(e.to_string()),
            Db(_) => ApiError::Internal(e.to_string()),
        }
    }
}

impl From<mb_governance::GovernanceError> for ApiError {
    fn from(e: mb_governance::GovernanceError) -> Self {
        use mb_governance::GovernanceError::*;
        match e {
            Db(_) => ApiError::Internal(e.to_string()),
        }
    }
}

impl From<mb_channels::ChannelRepoError> for ApiError {
    fn from(e: mb_channels::ChannelRepoError) -> Self {
        use mb_channels::ChannelRepoError::*;
        match e {
            NotFound(_) => ApiError::NotFound(e.to_string()),
            Db(_) => ApiError::Internal(e.to_string()),
        }
    }
}

impl From<mb_templates::TemplateError> for ApiError {
    fn from(e: mb_templates::TemplateError) -> Self {
        use mb_templates::TemplateError::*;
        match e {
            NotFound(_) | NoVersions(_) => ApiError::NotFound(e.to_string()),
            Db(_) => ApiError::Internal(e.to_string()),
        }
    }
}

impl From<mb_journeys::JourneyError> for ApiError {
    fn from(e: mb_journeys::JourneyError) -> Self {
        use mb_journeys::JourneyError::*;
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

impl From<mb_events::EventsError> for ApiError {
    fn from(e: mb_events::EventsError) -> Self {
        ApiError::Internal(e.to_string())
    }
}
