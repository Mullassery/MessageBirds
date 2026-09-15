//! Audience engine: rule-based segmentation. Conditions compose attribute
//! comparisons (on profile mixin fields) and event-occurrence checks with
//! AND/OR/NOT (Section 20). Membership is evaluated in real time by the
//! worker as a profile's mixins change (Section 21) and synced to Postgres
//! — not round-tripped through Kafka as synthetic events, which is a
//! separate, deferred piece of work. Sequence conditions ("A then B then
//! not C") are a different evaluation model and are not implemented here.

mod eval;
mod model;
mod repo;

pub use eval::{eval_attribute_op, get_field};
pub use model::{
    AttributeOp, AudienceDefinition, AudienceStatus, Condition, Membership, MembershipChange,
    MembershipEvent, MembershipKind,
};
pub use repo::{AudienceError, AudienceRepo, PgAudienceRepo};
