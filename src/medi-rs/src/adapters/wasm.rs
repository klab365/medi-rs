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
