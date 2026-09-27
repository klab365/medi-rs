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

impl_event_queue!(
    WasmEventQueue,
    sender: mpsc::Sender<T>,
    receiver: mpsc::Receiver<T>,
    channel: |capacity| mpsc::channel(capacity.unwrap_or(1024)),
    send: |sender, item| sender.clone().send(item),
    try_send: |sender, item| {
        let mut sender = sender.clone();
        sender.try_send(item).map_err(|error| {
            let is_full = error.is_full();
            let item = error.into_inner();
            if is_full {
                crate::TryPublishError::Full(item)
            } else {
                crate::TryPublishError::Closed(item)
            }
        })
    },
    receive: |receiver| receiver.next().await,
);

/// WebAssembly stream channel: a bounded futures `mpsc` channel created with
/// the mediator.
pub struct WasmStreamChannel<T> {
    // A single locked sender keeps the channel bounded: every cloned futures
    // sender would receive its own guaranteed slot.
    sender: Mutex<mpsc::Sender<T>>,
    receiver: Mutex<mpsc::Receiver<T>>,
}

/// Handler-side send handle for [`WasmStreamChannel`].
pub struct WasmStreamSender<'a, T>(&'a Mutex<mpsc::Sender<T>>);

impl<T> Clone for WasmStreamSender<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for WasmStreamSender<'_, T> {}

/// Consumer-side receive handle for [`WasmStreamChannel`].
pub struct WasmStreamReceiver<'a, T>(futures::lock::MutexGuard<'a, mpsc::Receiver<T>>);

impl<T: Send + 'static> crate::StreamChannel<T> for WasmStreamChannel<T> {
    type Sender<'a> = WasmStreamSender<'a, T>;
    type Receiver<'a> = WasmStreamReceiver<'a, T>;

    fn new(capacity: usize) -> Self {
        // A futures channel reserves one extra slot per sender, so subtract the
        // single mediator-owned sender to honor the requested capacity.
        let (sender, receiver) = mpsc::channel(capacity.saturating_sub(1));
        Self {
            sender: Mutex::new(sender),
            receiver: Mutex::new(receiver),
        }
    }

    async fn open(&self) -> (Self::Sender<'_>, Self::Receiver<'_>) {
        let mut receiver = self.receiver.lock().await;
        // Discard items left behind by a cancelled stream.
        while receiver.try_recv().is_ok() {}
        (WasmStreamSender(&self.sender), WasmStreamReceiver(receiver))
    }
}

impl<T: Send> crate::StreamChannelSender<T> for WasmStreamSender<'_, T> {
    async fn send(&self, item: T) {
        let _ = self.0.lock().await.send(item).await;
    }
}

impl<T: Send> crate::StreamChannelReceiver<T> for WasmStreamReceiver<'_, T> {
    async fn recv(&mut self) -> Option<T> {
        self.0.next().await
    }
}
