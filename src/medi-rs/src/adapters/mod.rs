//! Runtime queue and task-spawn adapters for generated mediators.

#[allow(unused_macros)] // No runtime adapter is enabled in the default build.
macro_rules! impl_event_queue {
    (
        $queue:ident,
        sender: $sender:ty,
        sender_binding: $sender_name:ident [$($sender_mut:tt)*],
        receiver: $receiver:ty,
        channel: |$capacity:ident| $channel:expr,
        receive: |$receiver_name:ident| $receive:expr $(,)?
    ) => {
        pub struct $queue<T> {
            sender: Mutex<($sender, bool)>,
            receiver: Mutex<$receiver>,
        }

        impl<T: Send + 'static> crate::EventQueue<T> for $queue<T> {
            fn new($capacity: Option<usize>) -> Self {
                let (sender, receiver) = $channel;
                Self {
                    sender: Mutex::new((sender, false)),
                    receiver: Mutex::new(receiver),
                }
            }

            async fn publish(&self, item: T) -> crate::Result<()> {
                let $($sender_mut)* $sender_name = self.sender.lock().await;
                if $sender_name.1 {
                    return Err(crate::Error::EventPublishingError);
                }
                $sender_name
                    .0
                    .send(item)
                    .await
                    .map_err(|_| crate::Error::EventPublishingError)
            }

            async fn close(&self) {
                self.sender.lock().await.1 = true;
            }

            async fn publish_internal(&self, item: T) -> crate::Result<()> {
                let $($sender_mut)* $sender_name = self.sender.lock().await;
                $sender_name
                    .0
                    .send(item)
                    .await
                    .map_err(|_| crate::Error::EventPublishingError)
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
