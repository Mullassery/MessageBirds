use sqlx::PgPool;
use std::sync::Arc;

use mb_events::EventProducer;
use mb_identity::PgIdentityRepo;
use mb_mixins::PgMixinRepo;
use mb_namespaces::PgNamespaceRepo;
use mb_profile::PgProfileRepo;
use mb_schema_registry::PgSchemaRepo;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub producer: Arc<EventProducer>,
    pub schemas: Arc<PgSchemaRepo>,
    pub mixins: Arc<PgMixinRepo>,
    pub namespaces: Arc<PgNamespaceRepo>,
    pub profiles: Arc<PgProfileRepo>,
    pub identity: Arc<PgIdentityRepo>,
}
