//! Unified customer profile: a projection composed from mixins, built by
//! applying the tenant's merge policy field-by-field as events arrive.
//! Profiles are derived state, not the source of truth — that's the event
//! stream (Section 9).

mod decide;
mod model;
mod repo;
mod similarity;

pub use decide::{decide, Decision};
pub use model::{FieldProvenance, Profile};
pub use repo::{PgProfileRepo, ProfileError, ProfileRepo};
pub use similarity::{score, PersonSignal};
