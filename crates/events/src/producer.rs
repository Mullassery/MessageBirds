use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::util::Timeout;
use std::time::Duration;

use mb_core::EventEnvelope;

use crate::error::EventsError;

pub struct EventProducer {
    producer: FutureProducer,
}

impl EventProducer {
    pub fn new(brokers: &str) -> Result<Self, EventsError> {
        let producer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("message.timeout.ms", "5000")
            .create()?;
        Ok(Self { producer })
    }

    pub async fn publish(&self, topic: &str, envelope: &EventEnvelope) -> Result<(), EventsError> {
        let payload = serde_json::to_vec(envelope)?;
        let key = partition_key(envelope);

        self.producer
            .send(
                FutureRecord::to(topic).key(&key).payload(&payload),
                Timeout::After(Duration::from_secs(5)),
            )
            .await
            .map_err(|(err, _)| EventsError::Kafka(err))?;

        Ok(())
    }
}

/// Keeps one customer's events on one partition (and therefore in order)
/// by keying on tenant + their primary identity claim, falling back to the
/// first claim if none is marked primary.
fn partition_key(envelope: &EventEnvelope) -> String {
    let identity = envelope
        .identity
        .iter()
        .find(|i| i.primary)
        .or_else(|| envelope.identity.first());

    match identity {
        Some(id) => format!("{}:{}:{}", envelope.tenant_id, id.namespace, id.value),
        None => envelope.tenant_id.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use mb_core::{EventSource, IdentityRef, SchemaRef, TenantId};
    use uuid::Uuid;

    fn envelope(identities: Vec<IdentityRef>) -> EventEnvelope {
        EventEnvelope {
            event_id: Uuid::new_v4(),
            event_type: "commerce.product_view".into(),
            timestamp: Utc::now(),
            source: EventSource {
                kind: "web".into(),
                name: "website".into(),
            },
            identity: identities,
            context: serde_json::json!({}),
            data: serde_json::json!({}),
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
    fn keys_by_primary_identity_when_present() {
        let e = envelope(vec![
            IdentityRef {
                namespace: "anonymous_id".into(),
                value: "a1".into(),
                primary: false,
                source: "web".into(),
                confidence: 1.0,
            },
            IdentityRef {
                namespace: "email".into(),
                value: "j@example.com".into(),
                primary: true,
                source: "web".into(),
                confidence: 1.0,
            },
        ]);
        assert_eq!(
            partition_key(&e),
            format!("{}:email:j@example.com", e.tenant_id)
        );
    }

    #[test]
    fn falls_back_to_first_identity() {
        let e = envelope(vec![IdentityRef {
            namespace: "anonymous_id".into(),
            value: "a1".into(),
            primary: false,
            source: "web".into(),
            confidence: 1.0,
        }]);
        assert_eq!(
            partition_key(&e),
            format!("{}:anonymous_id:a1", e.tenant_id)
        );
    }
}
