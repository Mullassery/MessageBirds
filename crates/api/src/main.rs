mod auth;
mod error;
mod http_source;
mod labels;
mod routes;
mod squid_source;
mod state;

use std::sync::Arc;

use sqlx::postgres::PgPoolOptions;

use pch_audiences::PgAudienceRepo;
use pch_channels::{PgChannelRepo, WebhookChannelAdapter};
use pch_connectors::{HubspotConnector, KlaviyoConnector, PgConnectorRepo, WebhookConnector};
use pch_events::EventProducer;
use pch_governance::PgGovernanceRepo;
use pch_identity::PgIdentityRepo;
use pch_journeys::{Engine as JourneyEngine, PgJourneyRepo};
use pch_mixins::PgMixinRepo;
use pch_namespaces::PgNamespaceRepo;
use pch_profile::PgProfileRepo;
use pch_schema_registry::PgSchemaRepo;
use pch_templates::PgTemplateRepo;

use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://pycustomerhub:pycustomerhub@localhost:5432/pycustomerhub".into()
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
        hubspot: Arc::new(HubspotConnector::new()),
        klaviyo: Arc::new(KlaviyoConnector::new()),
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
        auth: auth::ApiKeyAuth::from_env(),
        http_write_keys: http_source::HttpWriteKeys::from_env(),
        squid_workspaces: squid_source::SquidWorkspaces::from_env(),
    };

    let app = routes::router(state);

    // Secure by default: bind to loopback only. The API enforces no
    // authentication, so exposing it on a routable interface hands every
    // tenant's data to anyone who can reach the port. Operators who really
    // need that (e.g. behind an authenticating proxy or in a trusted
    // network) must opt in explicitly.
    let allow_non_loopback = std::env::var("PYCUSTOMERHUB_ALLOW_NON_LOOPBACK")
        .or_else(|_| std::env::var("MESSAGEBIRDS_ALLOW_NON_LOOPBACK"))
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let bind_host = if allow_non_loopback {
        "0.0.0.0"
    } else {
        "127.0.0.1"
    };
    let listener = tokio::net::TcpListener::bind((bind_host, port)).await?;
    tracing::info!("api listening on {bind_host}:{port}");
    if !allow_non_loopback {
        tracing::warn!(
            "API is bound to loopback only and enforces no auth; \
             set PYCUSTOMERHUB_ALLOW_NON_LOOPBACK=1 to expose it"
        );
    }
    axum::serve(listener, app).await?;

    Ok(())
}
