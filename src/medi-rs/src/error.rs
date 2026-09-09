//! Errors returned by mediator lifecycle and typed event queue operations.

/// Error returned when a mediator is started more than once.
///
/// A mediator starts its generated event workers and `#[medi_task]` tasks at
/// most once. Check `is_started` before calling `start` when needed.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum StartError {
    /// The mediator has already started its generated workers and tasks.
    AlreadyStarted,
}

/// Error returned by a non-blocking event publish attempt.
///
/// The event is returned so callers can retry, persist, or discard it.
#[derive(Debug, Eq, PartialEq)]
pub enum TryPublishError<T> {
    /// The bounded queue has no available capacity.
    Full(T),
    /// The mediator is shutting down or its event worker is unavailable.
    Closed(T),
}

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
