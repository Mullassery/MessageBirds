//! Destinations and activation (Sections 18, 60): a small plugin shape
//! (`DestinationConnector`) with one real implementation, `webhook`. Every
//! activation attempt is recorded in `activation_log` — the audit trail
//! Section 60 requires, even without the governance/consent checks that
//! would normally gate it (those aren't implemented yet).

mod model;
mod repo;
mod webhook;

pub use model::{ActivationRecord, ActivationStatus, Destination, WEBHOOK_KIND};
pub use repo::{ActivationRepo, ConnectorRepoError, DestinationRepo, PgConnectorRepo};
pub use webhook::{ConnectorError, DestinationConnector, WebhookConnector};
