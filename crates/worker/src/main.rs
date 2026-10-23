mod identity_mixin;
mod persist;
mod pipeline;

use std::sync::Arc;

use sqlx::postgres::PgPoolOptions;

use pch_audiences::PgAudienceRepo;
use pch_channels::{PgChannelRepo, WebhookChannelAdapter};
use pch_events::EventConsumer;
use pch_identity::PgIdentityRepo;
use pch_journeys::{Engine as JourneyEngine, PgJourneyRepo};
use pch_merge_policy::PgMergePolicyRepo;
use pch_mixins::PgMixinRepo;
use pch_profile::PgProfileRepo;
use pch_schema_registry::PgSchemaRepo;
use pch_templates::PgTemplateRepo;

use pipeline::{Outcome, Pipeline};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://pycustomerhub:pycustomerhub@localhost:5432/pycustomerhub".into()
    });
    let kafka_brokers = std::env::var("KAFKA_BROKERS").unwrap_or_else(|_| "localhost:9092".into());
    let group_id =
        std::env::var("KAFKA_GROUP_ID").unwrap_or_else(|_| "pycustomerhub-worker".into());

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    let consumer = EventConsumer::new(&kafka_brokers, &group_id, pch_events::RAW_EVENTS_TOPIC)?;

    let journey_engine = JourneyEngine {
        pool: pool.clone(),
        profiles: Arc::new(PgProfileRepo::new(pool.clone())),
        templates: Arc::new(PgTemplateRepo::new(pool.clone())),
        channels: Arc::new(PgChannelRepo::new(pool.clone())),
        channel_adapter: Arc::new(WebhookChannelAdapter::new()),
    };

    let pipeline = Pipeline {
        pool: pool.clone(),
        schemas: Arc::new(PgSchemaRepo::new(pool.clone())),
        mixins: Arc::new(PgMixinRepo::new(pool.clone())),
        identity: Arc::new(PgIdentityRepo::new(pool.clone())),
        merge_policies: Arc::new(PgMergePolicyRepo::new(pool.clone())),
        profiles: Arc::new(PgProfileRepo::new(pool.clone())),
        audiences: Arc::new(PgAudienceRepo::new(pool.clone())),
        journeys: Arc::new(PgJourneyRepo::new(pool, journey_engine)),
    };

    tracing::info!("worker consuming '{}'", pch_events::RAW_EVENTS_TOPIC);

    loop {
        let received = match consumer.recv().await {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("kafka receive error: {e}");
                continue;
            }
        };

        match pipeline.process(&received.envelope).await {
            Ok(Outcome::Accepted { profile_id }) => {
                tracing::info!(event_id = %received.envelope.event_id, %profile_id, "event accepted");
            }
            Ok(Outcome::Rejected { reason }) => {
                tracing::warn!(event_id = %received.envelope.event_id, %reason, "event rejected to DLQ");
            }
            Err(e) => {
                tracing::error!(event_id = %received.envelope.event_id, error = %e, "processing failed, will redeliver");
                continue;
            }
        }

        if let Err(e) = consumer.commit(&received) {
            tracing::error!("failed to commit offset: {e}");
        }
    }
}
