use crate::{Error, Result, TryPublishError};
use core::future::Future;
use core::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{Mutex, mpsc};

pub fn spawn<F>(future: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    tokio::spawn(future);
}

/// Tokio-backed bounded event queue.
pub struct TokioEventQueue<T> {
    sender: mpsc::Sender<T>,
    receiver: Mutex<mpsc::Receiver<T>>,
    gate: Mutex<()>,
    closed: AtomicBool,
}

impl<T: Send + 'static> crate::EventQueue<T> for TokioEventQueue<T> {
    fn new(capacity: Option<usize>) -> Self {
        let (sender, receiver) = mpsc::channel(capacity.unwrap_or(1024));
        Self {
            sender,
            receiver: Mutex::new(receiver),
            gate: Mutex::new(()),
            closed: AtomicBool::new(false),
        }
    }

    async fn publish(&self, item: T) -> Result<()> {
        let _gate = self.gate.lock().await;
        if self.closed.load(Ordering::Acquire) {
            return Err(Error::EventPublishingError);
        }
        self.sender.send(item).await.map_err(|_| Error::EventPublishingError)
    }

    fn try_publish(&self, item: T) -> core::result::Result<(), TryPublishError<T>> {
        if self.closed.load(Ordering::Acquire) {
            return Err(TryPublishError::Closed(item));
        }
        self.sender.try_send(item).map_err(|error| match error {
            mpsc::error::TrySendError::Full(item) => TryPublishError::Full(item),
            mpsc::error::TrySendError::Closed(item) => TryPublishError::Closed(item),
        })
    }

    async fn close(&self) {
        let _gate = self.gate.lock().await;
        self.closed.store(true, Ordering::Release);
    }

    async fn publish_internal(&self, item: T) -> Result<()> {
        self.sender.send(item).await.map_err(|_| Error::EventPublishingError)
    }

    async fn recv(&self) -> Result<T> {
        self.receiver
            .lock()
            .await
            .recv()
            .await
            .ok_or(Error::EventProcessingError)
    }
}
