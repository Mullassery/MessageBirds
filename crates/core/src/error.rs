use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EnvelopeValidationError {
    #[error("event_type must not be empty")]
    EmptyEventType,
    #[error("schema name and version must not be empty")]
    EmptySchemaRef,
    #[error("at least one identity claim is required")]
    NoIdentity,
    #[error("identity claim {0} has an empty namespace or value")]
    InvalidIdentity(usize),
}
