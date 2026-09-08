use core::future::Future;
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
    sender_binding: sender [],
    receiver: mpsc::Receiver<T>,
    channel: |capacity| mpsc::channel(capacity.unwrap_or(1024)),
    receive: |receiver| receiver.recv().await,
);
