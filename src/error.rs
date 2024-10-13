use thiserror::Error;

use crate::HandlerError;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Handler not found")]
    HandlerNotFound,

    #[error("Cast error")]
    CastError,

    #[error(transparent)]
    Handler(HandlerError),

    #[error("Resource not found")]
    ResourceNotFound,
}

pub type Result<T> = core::result::Result<T, Error>;

// -- from implementation for Error
impl From<HandlerError> for Error {
    fn from(e: HandlerError) -> Self {
        Self::Handler(e)
    }
}
