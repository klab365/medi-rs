//! Item channel abstraction for generated stream routes.
//!
//! Each runtime adapter provides a [`StreamChannel`] backed by that runtime's
//! bounded channel. A generated mediator owns one channel per stream route.
//! The channel is created once with the mediator; opening a stream leases it
//! and does not allocate.

use core::future::Future;

/// Bounded, mediator-owned channel that carries items from a stream handler
/// to the stream returned by the generated `stream` method.
///
/// The runtime feature selects the implementation: Tokio uses
/// `tokio::sync::mpsc`, WebAssembly uses `futures::channel::mpsc`, and Embassy
/// uses `embassy_sync::channel::Channel`.
///
/// Only one stream can hold a channel at a time. [`Self::open`] waits until the
/// previous lease has been released and discards items that a cancelled stream
/// left behind.
pub trait StreamChannel<T: Send>: Send + Sync + 'static {
    /// Handler-side send handle borrowed from the channel.
    type Sender<'a>: StreamChannelSender<T> + Copy + 'a
    where
        Self: 'a;

    /// Consumer-side receive handle. It holds the channel lease until dropped.
    type Receiver<'a>: StreamChannelReceiver<T> + 'a
    where
        Self: 'a;

    /// Create a channel with room for `capacity` buffered items.
    ///
    /// Adapters with a compile-time capacity (Embassy) ignore the argument.
    fn new(capacity: usize) -> Self;

    /// Wait for exclusive use of the channel and return its send and receive
    /// handles.
    fn open(&self) -> impl Future<Output = (Self::Sender<'_>, Self::Receiver<'_>)>;
}

/// Send half of a leased [`StreamChannel`].
pub trait StreamChannelSender<T> {
    /// Send an item, waiting while the channel is full.
    ///
    /// The item is discarded if the receiver no longer exists.
    fn send(&self, item: T) -> impl Future<Output = ()>;
}

/// Receive half of a leased [`StreamChannel`].
pub trait StreamChannelReceiver<T> {
    /// Receive the next item, or `None` when the channel is closed.
    fn recv(&mut self) -> impl Future<Output = Option<T>>;
}
