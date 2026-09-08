use crate::{Error, Result, TryPublishError};
use core::sync::atomic::{AtomicBool, Ordering};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;

/// Embassy queue whose capacity is selected by the generated mediator type.
pub struct EmbassyEventQueue<T: 'static, const CAPACITY: usize> {
    channel: Channel<CriticalSectionRawMutex, T, CAPACITY>,
    closed: AtomicBool,
}
impl<T: Send + 'static, const CAPACITY: usize> crate::EventQueue<T> for EmbassyEventQueue<T, CAPACITY> {
    fn new(_: Option<usize>) -> Self {
        Self {
            channel: Channel::new(),
            closed: AtomicBool::new(false),
        }
    }
    async fn publish(&self, item: T) -> Result<()> {
        if self.closed.load(Ordering::Acquire) {
            return Err(Error::EventPublishingError);
        }
        self.channel.send(item).await;
        if self.closed.load(Ordering::Acquire) {
            // The item was accepted before shutdown and must be drained.
        }
        Ok(())
    }
    fn try_publish(&self, item: T) -> core::result::Result<(), TryPublishError<T>> {
        if self.closed.load(Ordering::Acquire) {
            return Err(TryPublishError::Closed(item));
        }
        self.channel.try_send(item).map_err(|error| match error {
            embassy_sync::channel::TrySendError::Full(item) => TryPublishError::Full(item),
        })
    }
    async fn close(&self) {
        self.closed.store(true, Ordering::Release);
    }
    async fn publish_internal(&self, item: T) -> Result<()> {
        self.channel.send(item).await;
        Ok(())
    }
    async fn recv(&self) -> Result<T> {
        Ok(self.channel.receive().await)
    }
}
