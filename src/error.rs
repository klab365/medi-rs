use thiserror::Error;

use crate::HandlerError;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Handler not found")]
    HandlerNotFound,

    #[error("Cast error")]
    CastError,

    #[error(transparent)]
    Handler(#[from] HandlerError),

    #[error("Resource not found")]
    ResourceNotFound,

    #[error("No event handler registered")]
    NoEventHandlerRegistered,

    #[error("Event Processing Error")]
    EventProcessingError,
}

pub type Result<T> = core::result::Result<T, Error>;
