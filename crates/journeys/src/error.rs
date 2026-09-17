use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum JourneyError {
    #[error("journey {0} is not registered")]
    NotFound(Uuid),
    #[error("journey run {0} is not registered")]
    RunNotFound(Uuid),
    #[error("node '{0}' does not exist in this journey")]
    UnknownNode(String),
    #[error("profile {0} not found")]
    ProfileNotFound(Uuid),
    #[error("stored journey nodes failed to deserialize: {0}")]
    Corrupt(#[from] serde_json::Error),
    #[error(transparent)]
    Audience(#[from] mb_audiences::AudienceError),
    #[error(transparent)]
    Profile(#[from] mb_profile::ProfileError),
    #[error(transparent)]
    Template(#[from] mb_templates::TemplateError),
    #[error(transparent)]
    Channel(#[from] mb_channels::ChannelRepoError),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}
