use crate::{Error, Result, TryPublishError};
use core::sync::atomic::{AtomicBool, Ordering};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::{Channel, SendDynamicReceiver, SendDynamicSender};
use embassy_sync::mutex::{Mutex, MutexGuard};

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

/// Embassy stream channel whose capacity is selected by the stream request.
pub struct EmbassyStreamChannel<T: 'static, const CAPACITY: usize> {
    channel: Channel<CriticalSectionRawMutex, T, CAPACITY>,
    lease: Mutex<CriticalSectionRawMutex, ()>,
}

/// Handler-side send handle for [`EmbassyStreamChannel`].
///
/// It is independent of the channel capacity, so handlers can name it without
/// a const generic.
pub struct EmbassyStreamSender<'a, T>(SendDynamicSender<'a, T>);

impl<T> Clone for EmbassyStreamSender<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for EmbassyStreamSender<'_, T> {}

/// Consumer-side receive handle for [`EmbassyStreamChannel`].
pub struct EmbassyStreamReceiver<'a, T> {
    receiver: SendDynamicReceiver<'a, T>,
    _lease: MutexGuard<'a, CriticalSectionRawMutex, ()>,
}

impl<T: Send + 'static, const CAPACITY: usize> crate::StreamChannel<T> for EmbassyStreamChannel<T, CAPACITY> {
    type Sender<'a> = EmbassyStreamSender<'a, T>;
    type Receiver<'a> = EmbassyStreamReceiver<'a, T>;

    fn new(_: usize) -> Self {
        Self {
            channel: Channel::new(),
            lease: Mutex::new(()),
        }
    }

    async fn open(&self) -> (Self::Sender<'_>, Self::Receiver<'_>) {
        let lease = self.lease.lock().await;
        // Discard items left behind by a cancelled stream.
        self.channel.clear();
        (
            EmbassyStreamSender(self.channel.sender().into()),
            EmbassyStreamReceiver {
                receiver: self.channel.receiver().into(),
                _lease: lease,
            },
        )
    }
}

impl<T: Send> crate::StreamChannelSender<T> for EmbassyStreamSender<'_, T> {
    async fn send(&self, item: T) {
        self.0.send(item).await;
    }
}

impl<T: Send> crate::StreamChannelReceiver<T> for EmbassyStreamReceiver<'_, T> {
    async fn recv(&mut self) -> Option<T> {
        Some(self.receiver.receive().await)
    }
}
