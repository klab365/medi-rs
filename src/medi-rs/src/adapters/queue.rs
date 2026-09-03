//! Event/job queue abstraction for generated mediators.
//!
//! Each runtime adapter provides an implementation of [`EventQueue`] so that
//! the [`mediator!`](crate::mediator!) macro can enqueue and
//! dequeue typed events without coupling to a specific channel backend.

use crate::Result;
use core::future::Future;

/// Async queue used by generated mediators to enqueue events and dequeue
/// them in a background processing loop.
///
/// # Type parameters
///
/// * `T` — The job/enum type that the generated mediator uses to represent
///   all possible events across its subscribed modules.
pub trait EventQueue<T>: Send + Sync + 'static {
    /// Create a new queue.
    ///
    /// `Some(capacity)` requests a bounded queue; `None` requests an
    /// unbounded queue. Adapters may ignore the hint if the underlying
    /// channel does not support bounding.
    fn new(capacity: Option<usize>) -> Self;

    /// Enqueue an item.
    ///
    /// Returns [`Error::EventPublishingError`](crate::Error::EventPublishingError)
    /// if the channel is closed or full.
    fn publish(&self, item: T) -> impl Future<Output = Result<()>> + Send;

    /// Dequeue the next item.
    ///
    /// Returns [`Error::EventProcessingError`](crate::Error::EventProcessingError)
    /// if the channel is closed.
    fn recv(&self) -> impl Future<Output = Result<T>> + Send;
}
