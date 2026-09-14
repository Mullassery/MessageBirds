//! Merge policy engine: resolves conflicting field values from different
//! sources into one profile value (Section 10). Only `SOURCE_PRIORITY` and
//! `LATEST_TIMESTAMP` are implemented this phase.

mod apply;
mod model;
mod repo;

pub use apply::{apply_policy, MergePolicyError};
pub use model::{FieldCandidate, MergePolicy, Strategy};
pub use repo::{MergePolicyRepo, MergePolicyRepoError, PgMergePolicyRepo};
