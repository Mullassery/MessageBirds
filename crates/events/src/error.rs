use thiserror::Error;

#[derive(Debug, Error)]
pub enum EventsError {
    #[error("failed to serialize event envelope: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("kafka error: {0}")]
    Kafka(#[from] rdkafka::error::KafkaError),
    #[error("received a Kafka message with no payload")]
    EmptyPayload,
}
