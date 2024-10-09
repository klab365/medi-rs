use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Handler not found")]
    HandlerNotFound,

    #[error("Cast error")]
    CastError,

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Sync + Send + 'static>),

    #[error(transparent)]
    Resource(#[from] crate::resource::error::ResourceError),
}

pub type Result<T> = std::result::Result<T, Error>;
