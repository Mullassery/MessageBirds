//! Destinations and activation (Sections 18, 60): a small plugin shape
//! (`DestinationConnector`) with one real implementation, `webhook`. Every
//! activation attempt is recorded in `activation_log` — the audit trail
//! Section 60 requires, even without the governance/consent checks that
//! would normally gate it (those aren't implemented yet).

mod hubspot;
mod klaviyo;
mod model;
mod profile;
mod repo;
mod retry;
mod webhook;

pub use hubspot::HubspotConnector;
pub use klaviyo::KlaviyoConnector;
pub use model::{
    ActivationRecord, ActivationStatus, Destination, ExportRow, HUBSPOT_KIND, KLAVIYO_KIND,
    PYREVERSEETL_KIND, WEBHOOK_KIND,
};
pub use repo::{
    ActivationRepo, ConnectorRepoError, DestinationRepo, PgConnectorRepo, PyReverseEtlExportRepo,
};
pub use retry::{is_retryable, rate_limit_interval, send_with_retry, RetryPolicy};
pub use webhook::{ConnectorError, DestinationConnector, WebhookConnector};
