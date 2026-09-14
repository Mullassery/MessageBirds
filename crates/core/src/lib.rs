//! Canonical domain types shared across MessageBirds services.
//!
//! This crate has no I/O: it defines the shapes that flow through the
//! platform (events, identity references, schema/mixin references, tenancy
//! ids) so every other crate speaks the same language.

mod error;
mod event;
mod identity_ref;
mod ids;
mod mixin_ref;
mod schema_ref;

pub use error::EnvelopeValidationError;
pub use event::{EventContext, EventEnvelope, EventSource};
pub use identity_ref::IdentityRef;
pub use ids::{EnvironmentId, TenantId, WorkspaceId};
pub use mixin_ref::MixinRef;
pub use schema_ref::SchemaRef;
