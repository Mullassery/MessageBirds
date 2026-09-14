//! Identity namespace registry. Namespaces (`email`, `anonymous_id`,
//! `device_id`, ...) are registered data, never a hardcoded enum — new
//! namespaces can be added without a code change or redeploy.

mod model;
mod repo;

pub use model::{Namespace, NamespaceKind};
pub use repo::{NamespaceError, NamespaceRepo, PgNamespaceRepo};
