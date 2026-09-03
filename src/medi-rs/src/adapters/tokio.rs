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
    sender: mpsc::Sender<T>,
    receiver: Mutex<mpsc::Receiver<T>>,
}
impl<T: Send + 'static> crate::EventQueue<T> for TokioEventQueue<T> {
    fn new(capacity: Option<usize>) -> Self {
        let (sender, receiver) = mpsc::channel(capacity.unwrap_or(1024));
        Self {
            sender,
            receiver: Mutex::new(receiver),
        }
    }
    async fn publish(&self, item: T) -> Result<()> {
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
