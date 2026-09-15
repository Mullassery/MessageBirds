//! Governance: a label-based deny-list policy engine (Sections 13–16) and
//! an append-only consent ledger (Section 17). Policies are
//! `(label, action, effect, priority)` — default allow, an explicit `Deny`
//! blocks. No exceptions, inheritance, or policy versioning. Every
//! decision is explainable via structured `Reason`s, never a bare
//! "policy violation."

mod mapping;
mod model;
mod repo;

pub use mapping::consent_purpose_for_action;
pub use model::{ConsentEvent, ConsentState, Decision, Effect, Policy, Reason};
pub use repo::{ConsentRepo, GovernanceError, PgGovernanceRepo, PolicyRepo};
