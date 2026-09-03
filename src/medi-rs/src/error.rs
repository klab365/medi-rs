//! Errors returned by typed event queue operations.

/// Framework error for queue operations.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Error {
    /// An event could not be enqueued because the queue is closed.
    EventPublishingError,
    /// An event worker could not receive another event.
    EventProcessingError,
}

/// Framework result used by event publishing and queue operations.
pub type Result<T> = core::result::Result<T, Error>;
