//! Errors returned by mediator lifecycle and typed event queue operations.

/// Error returned when a mediator is started more than once.
///
/// A mediator starts its generated event workers and `#[medi_task]` tasks at
/// most once. Check `is_started` before calling `start` when needed.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum StartError {
    /// The mediator has already started its generated workers and tasks.
    AlreadyStarted,
    /// A startup hook returned an error.
    ///
    /// The hook name identifies the failed initialization step. Startup hooks
    /// may use unrelated concrete error types, so the generated mediator
    /// cannot expose one common error value.
    StartupHookFailed {
        /// Name of the hook that returned an error.
        hook: &'static str,
    },
}

/// Converts supported startup-hook return values into a mediator start result.
///
/// This trait is implemented for `()` and `Result<(), E>`. It is public only
/// because `#[medi_startup]` expands in the application crate.
#[doc(hidden)]
pub trait StartupHookResult {
    /// Return the framework start result for this hook invocation.
    fn into_start_result(self, hook: &'static str) -> core::result::Result<(), StartError>;
}

impl StartupHookResult for () {
    fn into_start_result(self, _: &'static str) -> core::result::Result<(), StartError> {
        Ok(())
    }
}

impl<E> StartupHookResult for core::result::Result<(), E> {
    fn into_start_result(self, hook: &'static str) -> core::result::Result<(), StartError> {
        self.map_err(|_| StartError::StartupHookFailed { hook })
    }
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
