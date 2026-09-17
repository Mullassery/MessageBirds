//! Message templates (Section 29): versioned, channel-typed, with
//! `{{mixin_key.field}}` variable substitution against a profile's
//! composed mixins.

mod model;
mod render;
mod repo;

pub use model::{MessageTemplate, RenderedTemplate};
pub use render::render;
pub use repo::{PgTemplateRepo, TemplateError, TemplateRepo};
