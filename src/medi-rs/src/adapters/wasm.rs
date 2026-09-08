use crate::{Error, Result};
use core::future::Future;
use futures::lock::Mutex;
use futures::{SinkExt, StreamExt, channel::mpsc};
use wasm_bindgen_futures::spawn_local;

pub fn spawn<F>(future: F)
where
    F: Future<Output = ()> + 'static,
{
    spawn_local(future);
}

pub struct WasmEventQueue<T> {
    sender: Mutex<(mpsc::Sender<T>, bool)>,
    receiver: Mutex<mpsc::Receiver<T>>,
}
impl<T: Send + 'static> crate::EventQueue<T> for WasmEventQueue<T> {
    fn new(capacity: Option<usize>) -> Self {
        let (sender, receiver) = mpsc::channel(capacity.unwrap_or(1024));
        Self {
            sender: Mutex::new((sender, false)),
            receiver: Mutex::new(receiver),
        }
    }
    async fn publish(&self, item: T) -> Result<()> {
        let mut sender = self.sender.lock().await;
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
            .next()
            .await
            .ok_or(Error::EventProcessingError)
    }
}
