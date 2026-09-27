//! Typed streaming requests.
//!
//! A stream request derives [`MediStreamRequest`](crate::MediStreamRequest)
//! and is handled by a [`medi_stream_handler`](crate::medi_stream_handler)
//! function. The handler pushes items through a `StreamSender`; the
//! generated mediator's `stream` method returns them as a [`Stream`].
//!
//! # Semantics
//!
//! - The stream is lazy: the handler starts on the first poll and only makes
//!   progress while the stream is polled. No task is spawned.
//! - Every item sent by the handler is yielded as `Ok(item)`, in send order.
//!   `StreamSender::send` waits while the route's bounded channel is full.
//! - If the handler returns `Err(error)`, the stream yields every previously
//!   sent item, then exactly one `Err(error)`, then ends.
//! - If the handler returns `Ok(())`, the stream ends after the sent items.
//! - Dropping the stream cancels the handler at its current `.await` point and
//!   discards buffered items.
//! - Each stream route owns one channel in the mediator. While one stream of a
//!   route is active, another stream of the same route waits at its first poll
//!   until the active stream ends or is dropped.

pub use futures::stream::{Stream, StreamExt, TryStreamExt};

/// Metadata for a request whose handler produces a stream of items.
///
/// Most users should derive this with `#[derive(MediStreamRequest)]`.
pub trait StreamRequest: Send + 'static {
    /// Item type yielded by the stream.
    type Item: Send + 'static;

    /// Error type returned by the stream handler.
    type Error: Send + 'static;

    /// Number of items the route's channel buffers before
    /// `StreamSender::send` waits for the consumer. Always greater than zero.
    const CAPACITY: usize;
}

/// Message carried by a stream route's channel.
#[doc(hidden)]
pub enum StreamMessage<T, E> {
    /// An item produced by the handler.
    Item(T),
    /// The handler returned an error.
    Failed(E),
    /// The handler returned successfully.
    Completed,
}

#[cfg(any(feature = "tokio", feature = "wasm", feature = "embassy"))]
pub use runtime::*;

#[cfg(any(feature = "tokio", feature = "wasm", feature = "embassy"))]
mod runtime {
    use super::{StreamMessage, StreamRequest};
    use crate::adapters::selected::RawStreamSender;
    use crate::{StreamChannel, StreamChannelReceiver, StreamChannelSender};
    use core::future::{Future, ready};
    use futures::stream::{self, Stream, StreamExt};

    type Message<R> = StreamMessage<<R as StreamRequest>::Item, <R as StreamRequest>::Error>;

    /// Handle passed to a stream handler for producing items.
    ///
    /// Declare it as the parameter immediately before the request, for example
    /// `sender: StreamSender<'_, SearchUsers>`. It is `Copy`; dropping it does
    /// not end the stream—returning from the handler does.
    pub struct StreamSender<'a, R: StreamRequest> {
        raw: RawStreamSender<'a, Message<R>>,
    }

    impl<R: StreamRequest> Clone for StreamSender<'_, R> {
        fn clone(&self) -> Self {
            *self
        }
    }

    impl<R: StreamRequest> Copy for StreamSender<'_, R> {}

    impl<R: StreamRequest> StreamSender<'_, R> {
        /// Send one item to the stream consumer.
        ///
        /// Waits while the route's bounded channel is full. When the consumer
        /// drops the stream, the handler is cancelled at this `.await` point.
        pub async fn send(&self, item: R::Item) {
            self.raw.send(StreamMessage::Item(item)).await;
        }
    }

    /// Static stream route generated for a request and a concrete mediator.
    ///
    /// This is implemented by `mediator!`; applications call the generated
    /// mediator's inherent `stream` method instead.
    #[doc(hidden)]
    pub trait StaticStream<M>: StreamRequest + Sized {
        /// Open this request's generated stream route.
        fn stream(self, mediator: &M) -> impl Stream<Item = Result<Self::Item, Self::Error>> + '_;
    }

    /// Build the stream for one generated route.
    ///
    /// The handler future and the channel receiver are polled by the returned
    /// stream; nothing is spawned or allocated.
    #[doc(hidden)]
    pub fn open<'a, R, C, F, Fut>(channel: &'a C, handler: F) -> impl Stream<Item = Result<R::Item, R::Error>> + 'a
    where
        R: StreamRequest,
        C: StreamChannel<Message<R>, Sender<'a> = RawStreamSender<'a, Message<R>>>,
        F: FnOnce(StreamSender<'a, R>) -> Fut + 'a,
        Fut: Future<Output = Result<(), R::Error>> + 'a,
    {
        stream::once(async move {
            let (sender, receiver) = channel.open().await;
            let producer = async move {
                let outcome = match handler(StreamSender { raw: sender }).await {
                    Ok(()) => StreamMessage::Completed,
                    Err(error) => StreamMessage::Failed(error),
                };
                // The outcome is queued behind every sent item, which keeps
                // the error ordered after them.
                sender.send(outcome).await;
            };
            let items = stream::unfold(Some(receiver), |receiver| async move {
                let mut receiver = receiver?;
                match receiver.recv().await? {
                    StreamMessage::Item(item) => Some((Ok(item), Some(receiver))),
                    StreamMessage::Failed(error) => Some((Err(error), None)),
                    StreamMessage::Completed => None,
                }
            });
            let producer = stream::once(producer).filter_map(|()| ready(None));
            stream::select(items, producer)
        })
        .flatten()
    }
}
