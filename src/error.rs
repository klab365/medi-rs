use std::{any::Any, fmt::Display};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Handler not found")]
    HandlerNotFound,

    #[error("Cast error")]
    CastError,

    #[error("Handler error")]
    Handler(Box<dyn Any + Send + Sync>),

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
    pub fn get_handler_error<T: Any + Send + Sync>(&self) -> Option<&T> {
        match self {
            Error::Handler(handler_error) => {
                handler_error.downcast_ref::<T>()
            },
            _ => None,
        }
    }
}

/// Trait to convert a type into a handler error
pub trait IntoHandlerError
where
    Self: Sized + Send + Sync + 'static,
{
    fn into_handler_error(self) -> Error {
        Error::Handler(Box::new(self))
    }
}

impl IntoHandlerError for Error {
    fn into_handler_error(self) -> Error {
        self
    }
}

impl IntoHandlerError for String {
	fn into_handler_error(self) -> Error {
		Error::Handler(Box::new(self))
	}
}

impl IntoHandlerError for &'static str {
	fn into_handler_error(self) -> Error {
		Error::Handler(Box::new(self))
	}
}


#[cfg(test)]
mod tests {
    use crate::{Error, IntoHandlerError};

    #[test]
    fn test_handler_error() {
        let error = "error".into_handler_error();
        assert_eq!(error.get_handler_error::<&str>(), Some(&"error"));
        assert_eq!(error.get_handler_error::<i32>(), None);
    }

    #[allow(clippy::useless_conversion)]
    #[test]
    fn into_handler_error_should_return_handler_error_when_handler_error() {
        let err: Error = "error".into_handler_error();
        assert_eq!(err.get_handler_error::<&str>(), Some(&"error"));
    }
}
