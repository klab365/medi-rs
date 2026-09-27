use core::future::Future;
use core::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{Mutex, mpsc};

pub fn spawn<F>(future: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    tokio::spawn(future);
}

impl_event_queue!(
    TokioEventQueue,
    sender: mpsc::Sender<T>,
    receiver: mpsc::Receiver<T>,
    channel: |capacity| mpsc::channel(capacity.unwrap_or(1024)),
    send: |sender, item| sender.send(item),
    try_send: |sender, item| sender.try_send(item).map_err(|error| match error {
        mpsc::error::TrySendError::Full(item) => crate::TryPublishError::Full(item),
        mpsc::error::TrySendError::Closed(item) => crate::TryPublishError::Closed(item),
    }),
    receive: |receiver| receiver.recv().await,
);

/// Tokio stream channel: a bounded `mpsc` channel created with the mediator.
pub struct TokioStreamChannel<T> {
    sender: mpsc::Sender<T>,
    receiver: Mutex<mpsc::Receiver<T>>,
}

/// Handler-side send handle for [`TokioStreamChannel`].
pub struct TokioStreamSender<'a, T>(&'a mpsc::Sender<T>);

impl<T> Clone for TokioStreamSender<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for TokioStreamSender<'_, T> {}

/// Consumer-side receive handle for [`TokioStreamChannel`].
pub struct TokioStreamReceiver<'a, T>(tokio::sync::MutexGuard<'a, mpsc::Receiver<T>>);

impl<T: Send + 'static> crate::StreamChannel<T> for TokioStreamChannel<T> {
    type Sender<'a> = TokioStreamSender<'a, T>;
    type Receiver<'a> = TokioStreamReceiver<'a, T>;

    fn new(capacity: usize) -> Self {
        let (sender, receiver) = mpsc::channel(capacity);
        Self {
            sender,
            receiver: Mutex::new(receiver),
        }
    }

    async fn open(&self) -> (Self::Sender<'_>, Self::Receiver<'_>) {
        let mut receiver = self.receiver.lock().await;
        // Discard items left behind by a cancelled stream.
        while receiver.try_recv().is_ok() {}
        (TokioStreamSender(&self.sender), TokioStreamReceiver(receiver))
    }
}

impl<T: Send> crate::StreamChannelSender<T> for TokioStreamSender<'_, T> {
    async fn send(&self, item: T) {
        let _ = self.0.send(item).await;
    }
}

impl<T: Send> crate::StreamChannelReceiver<T> for TokioStreamReceiver<'_, T> {
    async fn recv(&mut self) -> Option<T> {
        self.0.recv().await
    }
}
