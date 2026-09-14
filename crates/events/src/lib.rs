//! Kafka/Redpanda producer and consumer wrappers around the canonical
//! event envelope. Nothing in the platform reads or writes raw
//! `rdkafka` types outside this crate.

mod consumer;
mod error;
mod producer;
mod topics;

pub use consumer::{EventConsumer, ReceivedEvent};
pub use error::EventsError;
pub use producer::EventProducer;
pub use topics::RAW_EVENTS_TOPIC;
