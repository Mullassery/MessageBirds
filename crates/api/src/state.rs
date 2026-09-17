use sqlx::PgPool;
use std::sync::Arc;

use mb_audiences::PgAudienceRepo;
use mb_channels::PgChannelRepo;
use mb_connectors::{PgConnectorRepo, WebhookConnector};
use mb_events::EventProducer;
use mb_governance::PgGovernanceRepo;
use mb_identity::PgIdentityRepo;
use mb_journeys::PgJourneyRepo;
use mb_mixins::PgMixinRepo;
use mb_namespaces::PgNamespaceRepo;
use mb_profile::PgProfileRepo;
use mb_schema_registry::PgSchemaRepo;
use mb_templates::PgTemplateRepo;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub producer: Arc<EventProducer>,
    pub schemas: Arc<PgSchemaRepo>,
    pub mixins: Arc<PgMixinRepo>,
    pub namespaces: Arc<PgNamespaceRepo>,
    pub profiles: Arc<PgProfileRepo>,
    pub identity: Arc<PgIdentityRepo>,
    pub audiences: Arc<PgAudienceRepo>,
    /// Implements both `DestinationRepo` and `ActivationRepo` — one repo,
    /// two roles, since destinations and their activation log are closely
    /// related and there's no reason to split them into separate structs.
    pub connectors: Arc<PgConnectorRepo>,
    /// Implements both `PolicyRepo` and `ConsentRepo` — same rationale as
    /// `connectors` above.
    pub governance: Arc<PgGovernanceRepo>,
    pub webhook: Arc<WebhookConnector>,
    pub channels: Arc<PgChannelRepo>,
    pub templates: Arc<PgTemplateRepo>,
    /// Implements both `JourneyRepo` and `ContactPolicyRepo`.
    pub journeys: Arc<PgJourneyRepo>,
}
