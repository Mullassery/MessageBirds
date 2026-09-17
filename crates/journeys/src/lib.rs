//! Journey engine (Sections 24-27, 31): durable, Postgres-backed step
//! execution over a flat node graph. `journey_runs` persists
//! `(current_node, status, wake_at)` after every transition, so a crash
//! resumes exactly where it left off — real durability (survives
//! restart/crash), not distributed fault-tolerance across many worker
//! replicas, which this doesn't provide. A dedicated poller
//! (`journeys-worker`) drives `advance_due_runs`.

mod engine;
mod error;
mod model;
mod repo;
mod split;

pub use engine::Engine;
pub use error::JourneyError;
pub use model::{
    ContactPolicy, JourneyDefinition, JourneyRun, JourneyRunEvent, JourneyStatus, MessageSent,
    MessageStatus, Node, NodeKind, RunEventKind, RunStatus, SplitBranch, StepOutcome, Trigger,
};
pub use repo::{ContactPolicyRepo, JourneyRepo, PgJourneyRepo};
pub use split::choose_branch;
