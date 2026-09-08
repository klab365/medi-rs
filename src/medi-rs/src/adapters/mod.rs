//! Runtime queue and task-spawn adapters for generated mediators.

#[allow(unused_macros)] // No runtime adapter is enabled in the default build.
macro_rules! impl_event_queue {
    (
        $queue:ident,
        sender: $sender:ty,
        receiver: $receiver:ty,
        channel: |$capacity:ident| $channel:expr,
        send: |$send_sender:ident, $send_item:ident| $send:expr,
        try_send: |$try_sender:ident, $try_item:ident| $try_send:expr,
        receive: |$receiver_name:ident| $receive:expr $(,)?
    ) => {
        pub struct $queue<T> {
            sender: $sender,
            receiver: Mutex<$receiver>,
            gate: Mutex<()>,
            closed: AtomicBool,
        }

        impl<T: Send + 'static> crate::EventQueue<T> for $queue<T> {
            fn new($capacity: Option<usize>) -> Self {
                let (sender, receiver) = $channel;
                Self {
                    sender,
                    receiver: Mutex::new(receiver),
                    gate: Mutex::new(()),
                    closed: AtomicBool::new(false),
                }
            }

            async fn publish(&self, item: T) -> crate::Result<()> {
                let _gate = self.gate.lock().await;
                if self.closed.load(Ordering::Acquire) {
                    return Err(crate::Error::EventPublishingError);
                }
                let $send_sender = &self.sender;
                let $send_item = item;
                ($send).await.map_err(|_| crate::Error::EventPublishingError)
            }

            fn try_publish(&self, item: T) -> core::result::Result<(), crate::TryPublishError<T>> {
                if self.closed.load(Ordering::Acquire) {
                    return Err(crate::TryPublishError::Closed(item));
                }
                let $try_sender = &self.sender;
                let $try_item = item;
                $try_send
            }

            async fn close(&self) {
                let _gate = self.gate.lock().await;
                self.closed.store(true, Ordering::Release);
            }

            async fn publish_internal(&self, item: T) -> crate::Result<()> {
                let $send_sender = &self.sender;
                let $send_item = item;
                ($send).await.map_err(|_| crate::Error::EventPublishingError)
            }

            async fn recv(&self) -> crate::Result<T> {
                let mut $receiver_name = self.receiver.lock().await;
                ($receive).ok_or(crate::Error::EventProcessingError)
            }
        }
    };
}

pub mod lifecycle;
pub mod queue;
pub mod shutdown;

#[cfg(feature = "embassy")]
pub mod embassy;
#[cfg(feature = "tokio")]
pub mod tokio;
#[cfg(feature = "wasm")]
pub mod wasm;

#[cfg(all(feature = "tokio", not(feature = "wasm"), not(feature = "embassy")))]
pub mod selected {
    pub use super::tokio::TokioEventQueue as EventQueue;
    pub use super::tokio::spawn;
}
#[cfg(all(feature = "wasm", not(feature = "tokio"), not(feature = "embassy")))]
pub mod selected {
    pub use super::wasm::WasmEventQueue as EventQueue;
    pub use super::wasm::spawn;
}
#[cfg(all(feature = "embassy", not(feature = "tokio"), not(feature = "wasm")))]
pub mod selected {
    pub use super::embassy::EmbassyEventQueue as EventQueue;
}
