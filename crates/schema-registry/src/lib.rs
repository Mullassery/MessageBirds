//! First-class schema registry: field definitions, versioning, backward
//! compatibility checks, and payload validation. Connectors and SDKs do not
//! get to invent arbitrary customer JSON — everything that reaches a
//! profile is validated against a registered schema.

mod compatibility;
mod model;
mod repo;
mod validate;

pub use compatibility::check_backward_compatible;
pub use model::{FieldDef, FieldType, Schema, SchemaStatus};
pub use repo::{PgSchemaRepo, SchemaError, SchemaRepo};
pub use validate::{validate_fields, validate_payload, FieldViolation};
