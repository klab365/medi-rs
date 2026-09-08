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

impl_event_queue!(
    WasmEventQueue,
    sender: mpsc::Sender<T>,
    sender_binding: sender [mut],
    receiver: mpsc::Receiver<T>,
    channel: |capacity| mpsc::channel(capacity.unwrap_or(1024)),
    receive: |receiver| receiver.next().await,
);
