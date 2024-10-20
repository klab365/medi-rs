use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Handler not found")]
    HandlerNotFound,

    #[error("Cast error")]
    CastError,

    #[error("Handler error {0}")]
    Handler(Box<dyn std::error::Error + Send + Sync>),

    #[error("Resource not found")]
    ResourceNotFound,

    #[error("No event handler registered")]
    NoEventHandlerRegistered,

    #[error("Event Processing Error")]
    EventProcessingError,
}

/// Handler result type
/// This is a wrapper around the result type with the error type as the handler error
pub type HandlerResult<T> = core::result::Result<T, Error>;

impl Error {
    /// Get the handler error if it is a handler error
    pub fn get_handler_error<T: std::error::Error + Send + Sync + 'static>(&self) -> Option<&T> {
        match self {
            Error::Handler(handler_error) => handler_error.downcast_ref::<T>(),
            _ => None,
        }
    }
}

/// Trait to convert a type into a handler error
pub trait IntoHandlerError
where
    Self: std::error::Error + Sized + Send + Sync + 'static,
{
    fn into_handler_error(self) -> Error {
        Error::Handler(Box::new(self))
    }
}

impl IntoHandlerError for Error {}
