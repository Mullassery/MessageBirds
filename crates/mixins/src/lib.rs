//! Mixins: reusable, composable, versioned semantic blocks (`identity`,
//! `person`, `contact`, `device`, ...) that a profile is built out of.
//! Namespace ownership (`core/` for the standard library, anything else
//! for tenant-owned custom mixins) keeps custom mixins from corrupting the
//! canonical model.

mod compose;
mod model;
mod repo;
mod standard;

pub use compose::validate_mixin_fields;
pub use model::{MixinDef, MixinStatus};
pub use repo::{MixinError, MixinRepo, PgMixinRepo, STANDARD_NAMESPACE};
pub use standard::{standard_library, StandardMixinSpec};
