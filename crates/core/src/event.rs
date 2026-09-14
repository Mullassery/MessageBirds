use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{EnvelopeValidationError, IdentityRef, SchemaRef, TenantId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSource {
    #[serde(rename = "type")]
    pub kind: String,
    pub name: String,
}

/// Free-form situational context (device, application, location, ...).
/// Kept as a JSON blob rather than a fixed struct because it legitimately
/// varies by source type and is not part of the canonical validated model.
pub type EventContext = serde_json::Value;

/// The canonical, immutable event envelope. This is what crosses the wire
/// on `POST /events` and what gets published to Kafka — never the mutable
/// profile. Profiles are projections built *from* streams of these.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub event_id: Uuid,
    pub event_type: String,
    pub timestamp: DateTime<Utc>,
    pub source: EventSource,
    pub identity: Vec<IdentityRef>,
    #[serde(default)]
    pub context: EventContext,
    #[serde(default)]
    pub data: serde_json::Value,
    pub schema: SchemaRef,
    /// Captured verbatim; enforcement lives in the governance/consent
    /// planes, which are not implemented in this phase.
    #[serde(default)]
    pub consent: Option<serde_json::Value>,
    #[serde(default)]
    pub governance: Option<serde_json::Value>,
    #[serde(default)]
    pub correlation_id: Option<Uuid>,
    #[serde(default)]
    pub trace_id: Option<String>,
    pub tenant_id: TenantId,
}

impl EventEnvelope {
    /// Structural validation only: is this envelope well-formed enough to
    /// publish? Schema/mixin *content* validation happens downstream in the
    /// worker, against the schema registry.
    pub fn validate_shape(&self) -> Result<(), EnvelopeValidationError> {
        if self.event_type.trim().is_empty() {
            return Err(EnvelopeValidationError::EmptyEventType);
        }
        if self.schema.name.trim().is_empty() || self.schema.version.trim().is_empty() {
            return Err(EnvelopeValidationError::EmptySchemaRef);
        }
        if self.identity.is_empty() {
            return Err(EnvelopeValidationError::NoIdentity);
        }
        for (i, id) in self.identity.iter().enumerate() {
            if id.namespace.trim().is_empty() || id.value.trim().is_empty() {
                return Err(EnvelopeValidationError::InvalidIdentity(i));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SchemaRef;

    fn base_envelope() -> EventEnvelope {
        EventEnvelope {
            event_id: Uuid::new_v4(),
            event_type: "commerce.product_view".into(),
            timestamp: Utc::now(),
            source: EventSource {
                kind: "web".into(),
                name: "website".into(),
            },
            identity: vec![IdentityRef {
                namespace: "anonymous_id".into(),
                value: "abc123".into(),
                primary: true,
                source: "web-sdk".into(),
                confidence: 1.0,
            }],
            context: serde_json::json!({}),
            data: serde_json::json!({"product_id": "p1"}),
            schema: SchemaRef {
                name: "commerce.product_view".into(),
                version: "1.0".into(),
            },
            consent: None,
            governance: None,
            correlation_id: None,
            trace_id: None,
            tenant_id: TenantId::new(),
        }
    }

    #[test]
    fn accepts_well_formed_envelope() {
        assert!(base_envelope().validate_shape().is_ok());
    }

    #[test]
    fn rejects_missing_identity() {
        let mut e = base_envelope();
        e.identity.clear();
        assert_eq!(e.validate_shape(), Err(EnvelopeValidationError::NoIdentity));
    }

    #[test]
    fn rejects_empty_event_type() {
        let mut e = base_envelope();
        e.event_type = "  ".into();
        assert_eq!(
            e.validate_shape(),
            Err(EnvelopeValidationError::EmptyEventType)
        );
    }
}
