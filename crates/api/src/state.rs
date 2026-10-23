use sqlx::PgPool;
use std::sync::Arc;

use pch_audiences::PgAudienceRepo;
use pch_channels::PgChannelRepo;
use pch_connectors::{HubspotConnector, KlaviyoConnector, PgConnectorRepo, WebhookConnector};
use pch_events::EventProducer;
use pch_governance::PgGovernanceRepo;
use pch_identity::PgIdentityRepo;
use pch_journeys::PgJourneyRepo;
use pch_mixins::PgMixinRepo;
use pch_namespaces::PgNamespaceRepo;
use pch_profile::PgProfileRepo;
use pch_schema_registry::PgSchemaRepo;
use pch_templates::PgTemplateRepo;

use crate::auth::ApiKeyAuth;
use crate::http_source::HttpWriteKeys;
use crate::squid_source::SquidWorkspaces;

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
    pub hubspot: Arc<HubspotConnector>,
    pub klaviyo: Arc<KlaviyoConnector>,
    pub channels: Arc<PgChannelRepo>,
    pub templates: Arc<PgTemplateRepo>,
    /// Implements both `JourneyRepo` and `ContactPolicyRepo`.
    pub journeys: Arc<PgJourneyRepo>,
    /// Optional single-key API gate. Disabled when `PYCUSTOMERHUB_API_KEY` is unset.
    pub auth: ApiKeyAuth,
    /// Write-key -> tenant mapping for `/v1/*` tracking endpoints. These
    /// routes are deliberately outside the admin bearer-token gate; the
    /// write key is their authentication.
    pub http_write_keys: HttpWriteKeys,
    /// Squid workspace -> tenant/secret mapping for signed record webhooks.
    pub squid_workspaces: SquidWorkspaces,
}
