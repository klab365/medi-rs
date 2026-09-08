use core::future::Future;
use tokio::sync::{Mutex, mpsc};

use crate::{Error, Result};

pub fn spawn<F>(future: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    tokio::spawn(future);
}

pub struct TokioEventQueue<T> {
    sender: Mutex<(mpsc::Sender<T>, bool)>,
    receiver: Mutex<mpsc::Receiver<T>>,
}
impl<T: Send + 'static> crate::EventQueue<T> for TokioEventQueue<T> {
    fn new(capacity: Option<usize>) -> Self {
        let (sender, receiver) = mpsc::channel(capacity.unwrap_or(1024));
        Self {
            sender: Mutex::new((sender, false)),
            receiver: Mutex::new(receiver),
        }
    }
    async fn publish(&self, item: T) -> Result<()> {
        let sender = self.sender.lock().await;
        if sender.1 {
            return Err(Error::EventPublishingError);
        }
        sender.0.send(item).await.map_err(|_| Error::EventPublishingError)
    }
    async fn close(&self) {
        self.sender.lock().await.1 = true;
    }
    async fn publish_internal(&self, item: T) -> Result<()> {
        self.sender
            .lock()
            .await
            .0
            .send(item)
            .await
            .map_err(|_| Error::EventPublishingError)
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
