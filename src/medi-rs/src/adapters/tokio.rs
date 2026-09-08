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
