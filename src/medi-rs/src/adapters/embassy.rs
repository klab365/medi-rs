use crate::Result;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;

/// Embassy queue whose capacity is selected by the generated mediator type.
pub struct EmbassyEventQueue<T: 'static, const CAPACITY: usize> {
    channel: Channel<CriticalSectionRawMutex, T, CAPACITY>,
}
impl<T: Send + 'static, const CAPACITY: usize> crate::EventQueue<T> for EmbassyEventQueue<T, CAPACITY> {
    fn new(_: Option<usize>) -> Self {
        Self {
            channel: Channel::new(),
        }
    }
    async fn publish(&self, item: T) -> Result<()> {
        self.channel.send(item).await;
        Ok(())
    }
    async fn recv(&self) -> Result<T> {
        Ok(self.channel.receive().await)
    }
}
