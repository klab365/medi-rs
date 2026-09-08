use crate::{Error, Result, TryPublishError};
use core::future::Future;
use core::sync::atomic::{AtomicBool, Ordering};
use futures::lock::Mutex;
use futures::{SinkExt, StreamExt, channel::mpsc};
use wasm_bindgen_futures::spawn_local;

pub fn spawn<F>(future: F)
where
    F: Future<Output = ()> + 'static,
{
    spawn_local(future);
}

/// WebAssembly-backed bounded event queue.
pub struct WasmEventQueue<T> {
    sender: mpsc::Sender<T>,
    receiver: Mutex<mpsc::Receiver<T>>,
    gate: Mutex<()>,
    closed: AtomicBool,
}

impl<T: Send + 'static> crate::EventQueue<T> for WasmEventQueue<T> {
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
        self.sender
            .clone()
            .send(item)
            .await
            .map_err(|_| Error::EventPublishingError)
    }

    fn try_publish(&self, item: T) -> core::result::Result<(), TryPublishError<T>> {
        if self.closed.load(Ordering::Acquire) {
            return Err(TryPublishError::Closed(item));
        }
        let mut sender = self.sender.clone();
        sender.try_send(item).map_err(|error| {
            let is_full = error.is_full();
            let item = error.into_inner();
            if is_full {
                TryPublishError::Full(item)
            } else {
                TryPublishError::Closed(item)
            }
        })
    }

    async fn close(&self) {
        let _gate = self.gate.lock().await;
        self.closed.store(true, Ordering::Release);
    }

    async fn publish_internal(&self, item: T) -> Result<()> {
        self.sender
            .clone()
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
