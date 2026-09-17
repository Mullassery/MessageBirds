mod error;
mod labels;
mod routes;
mod state;

use std::sync::Arc;

use sqlx::postgres::PgPoolOptions;

use mb_audiences::PgAudienceRepo;
use mb_channels::{PgChannelRepo, WebhookChannelAdapter};
use mb_connectors::{PgConnectorRepo, WebhookConnector};
use mb_events::EventProducer;
use mb_governance::PgGovernanceRepo;
use mb_identity::PgIdentityRepo;
use mb_journeys::{Engine as JourneyEngine, PgJourneyRepo};
use mb_mixins::PgMixinRepo;
use mb_namespaces::PgNamespaceRepo;
use mb_profile::PgProfileRepo;
use mb_schema_registry::PgSchemaRepo;
use mb_templates::PgTemplateRepo;

use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://messagebirds:messagebirds@localhost:5432/messagebirds".into()
    });
    let kafka_brokers = std::env::var("KAFKA_BROKERS").unwrap_or_else(|_| "localhost:9092".into());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    sqlx::migrate!("../../migrations").run(&pool).await?;

    let mixins = Arc::new(PgMixinRepo::new(pool.clone()));
    mixins.seed_standard_library().await?;

    let state = AppState {
        pool: pool.clone(),
        producer: Arc::new(EventProducer::new(&kafka_brokers)?),
        schemas: Arc::new(PgSchemaRepo::new(pool.clone())),
        mixins,
        namespaces: Arc::new(PgNamespaceRepo::new(pool.clone())),
        profiles: Arc::new(PgProfileRepo::new(pool.clone())),
        identity: Arc::new(PgIdentityRepo::new(pool.clone())),
        audiences: Arc::new(PgAudienceRepo::new(pool.clone())),
        connectors: Arc::new(PgConnectorRepo::new(pool.clone())),
        governance: Arc::new(PgGovernanceRepo::new(pool.clone())),
        webhook: Arc::new(WebhookConnector::new()),
        channels: Arc::new(PgChannelRepo::new(pool.clone())),
        templates: Arc::new(PgTemplateRepo::new(pool.clone())),
        journeys: Arc::new(PgJourneyRepo::new(
            pool.clone(),
            JourneyEngine {
                pool: pool.clone(),
                profiles: Arc::new(PgProfileRepo::new(pool.clone())),
                templates: Arc::new(PgTemplateRepo::new(pool.clone())),
                channels: Arc::new(PgChannelRepo::new(pool.clone())),
                channel_adapter: Arc::new(WebhookChannelAdapter::new()),
            },
        )),
    };

    let app = routes::router(state);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    tracing::info!("api listening on :{port}");
    axum::serve(listener, app).await?;

    Ok(())
}
