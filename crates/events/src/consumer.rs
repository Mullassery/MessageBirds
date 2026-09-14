use rdkafka::config::ClientConfig;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::topic_partition_list::{Offset, TopicPartitionList};
use rdkafka::Message;

use mb_core::EventEnvelope;

use crate::error::EventsError;

/// A deserialized event plus enough to commit its offset once the caller
/// is done with it. We commit *after* processing (success or DLQ), not on
/// receive, so a crash mid-processing redelivers the event instead of
/// silently dropping it.
pub struct ReceivedEvent {
    pub envelope: EventEnvelope,
    topic: String,
    partition: i32,
    offset: i64,
}

pub struct EventConsumer {
    consumer: StreamConsumer,
}

impl EventConsumer {
    pub fn new(brokers: &str, group_id: &str, topic: &str) -> Result<Self, EventsError> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("group.id", group_id)
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "earliest")
            .create()?;
        consumer.subscribe(&[topic])?;
        Ok(Self { consumer })
    }

    pub async fn recv(&self) -> Result<ReceivedEvent, EventsError> {
        let msg = self.consumer.recv().await?;
        let payload = msg.payload().ok_or(EventsError::EmptyPayload)?;
        let envelope: EventEnvelope = serde_json::from_slice(payload)?;
        Ok(ReceivedEvent {
            envelope,
            topic: msg.topic().to_string(),
            partition: msg.partition(),
            offset: msg.offset(),
        })
    }

    pub fn commit(&self, received: &ReceivedEvent) -> Result<(), EventsError> {
        let mut tpl = TopicPartitionList::new();
        tpl.add_partition_offset(
            &received.topic,
            received.partition,
            Offset::Offset(received.offset + 1),
        )?;
        self.consumer.commit(&tpl, CommitMode::Sync)?;
        Ok(())
    }
}
