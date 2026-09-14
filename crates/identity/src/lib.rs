//! Identity graph: deterministic matching of identity claims to profiles,
//! with every link and merge recorded in an append-only audit trail.
//! Probabilistic matching is a later phase — this milestone is exact
//! (namespace, value) matching only.

mod hash;
mod model;
mod repo;
mod resolve;

pub use hash::hash_value;
pub use model::{IdentityAuditEntry, IdentityAuditKind, IdentityNode, ResolvedIdentity};
pub use repo::{IdentityError, IdentityRepo, PgIdentityRepo};
pub use resolve::{pick_canonical, profiles_to_merge};
