use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Handler not found")]
    HandlerNotFound,

    #[error("Serialization error")]
    SerializationError,

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Sync + Send + 'static>),
}

pub type Result<T> = std::result::Result<T, Error>;
