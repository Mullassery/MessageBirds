//! Identity graph: deterministic matching of identity claims to profiles,
//! with every link, merge, and split recorded in an append-only audit
//! trail. Resolution itself (`resolve_or_create`) is exact (namespace,
//! value) matching only — confidence-scored merge *suggestions* based on
//! non-identity signals live in `mb-profile::similarity` plus a query in
//! `mb-api`, since they read profile mixin data this crate doesn't have.

mod hash;
mod model;
mod repo;
mod resolve;

pub use hash::hash_value;
pub use model::{IdentityAuditEntry, IdentityAuditKind, IdentityNode, ResolvedIdentity};
pub use repo::{IdentityError, IdentityRepo, PgIdentityRepo};
pub use resolve::{pick_canonical, profiles_to_merge};
