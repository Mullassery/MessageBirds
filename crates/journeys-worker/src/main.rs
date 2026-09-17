use std::sync::Arc;
use std::time::Duration;

use sqlx::postgres::PgPoolOptions;

use mb_channels::{PgChannelRepo, WebhookChannelAdapter};
use mb_journeys::{Engine, JourneyRepo, PgJourneyRepo};
use mb_profile::PgProfileRepo;
use mb_templates::PgTemplateRepo;

/// A dedicated poll loop — a separate process from `api`/`worker`, since
/// journey advancement is time-driven (`Wait` nodes) independent of Kafka
/// arrivals. Durability comes from `journey_runs` being fully persisted
/// after every transition (see `mb_journeys::Engine`), not from this
/// process staying alive — killing and restarting it mid-run resumes
/// correctly.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://messagebirds:messagebirds@localhost:5432/messagebirds".into()
    });
    let poll_interval_ms: u64 = std::env::var("JOURNEYS_POLL_INTERVAL_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1000);

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    let engine = Engine {
        pool: pool.clone(),
        profiles: Arc::new(PgProfileRepo::new(pool.clone())),
        templates: Arc::new(PgTemplateRepo::new(pool.clone())),
        channels: Arc::new(PgChannelRepo::new(pool.clone())),
        channel_adapter: Arc::new(WebhookChannelAdapter::new()),
    };
    let repo = PgJourneyRepo::new(pool, engine);

    tracing::info!("journeys-worker polling every {poll_interval_ms}ms");

    loop {
        match repo.advance_due_runs().await {
            Ok(n) if n > 0 => tracing::info!("advanced {n} due run(s)"),
            Ok(_) => {}
            Err(e) => tracing::error!("advance_due_runs failed: {e}"),
        }
        tokio::time::sleep(Duration::from_millis(poll_interval_ms)).await;
    }
}
