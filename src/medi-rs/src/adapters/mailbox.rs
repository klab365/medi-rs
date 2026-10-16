//! Runtime-neutral bounded mailbox primitives for v4 module actors.
//!
//! Generated module actors use a mailbox to receive typed jobs. The selected
//! runtime supplies the queue implementation through [`EventQueue`]; this
//! adapter gives actor code command-oriented errors and a non-blocking `post`
//! operation without coupling generated code to a concrete runtime.

use crate::{DispatchError, EventQueue, PostError, TryPublishError};
use core::future::Future;

/// Bounded queue used by one generated module actor.
///
/// `post` must never wait for capacity. This is essential when a handler posts
/// work to its own module: waiting on a full mailbox would deadlock the actor.
pub trait Mailbox<T>: Send + Sync + 'static {
    /// Create a mailbox with the requested bounded capacity.
    fn new(capacity: usize) -> Self;

    /// Enqueue a job, waiting for capacity when necessary.
    fn send(&self, item: T) -> impl Future<Output = core::result::Result<(), DispatchError>> + Send;

    /// Enqueue a job without waiting for capacity.
    fn post(&self, item: T) -> core::result::Result<(), PostError<T>>;

    /// Close the mailbox and enqueue a final actor-control job after accepted work.
    fn stop(&self, item: T) -> impl Future<Output = core::result::Result<(), DispatchError>> + Send;

    /// Receive the next job for the owning actor.
    fn recv(&self) -> impl Future<Output = core::result::Result<T, DispatchError>> + Send;
}

impl<T: Send + 'static, Q> Mailbox<T> for Q
where
    Q: EventQueue<T>,
{
    fn new(capacity: usize) -> Self {
        EventQueue::new(Some(capacity))
    }

    async fn send(&self, item: T) -> core::result::Result<(), DispatchError> {
        EventQueue::publish(self, item)
            .await
            .map_err(|_| DispatchError::ModuleUnavailable)
    }

    fn post(&self, item: T) -> core::result::Result<(), PostError<T>> {
        EventQueue::try_publish(self, item).map_err(|error| match error {
            TryPublishError::Full(item) => PostError::MailboxFull(item),
            TryPublishError::Closed(item) => PostError::ModuleUnavailable(item),
        })
    }

    async fn stop(&self, item: T) -> core::result::Result<(), DispatchError> {
        EventQueue::close(self).await;
        EventQueue::publish_internal(self, item)
            .await
            .map_err(|_| DispatchError::ModuleUnavailable)
    }

    async fn recv(&self) -> core::result::Result<T, DispatchError> {
        EventQueue::recv(self)
            .await
            .map_err(|_| DispatchError::ModuleUnavailable)
    }
}

#[cfg(all(test, feature = "tokio"))]
mod tests {
    use super::Mailbox;
    use crate::{PostError, adapters::tokio::TokioEventQueue};

    #[tokio::test]
    async fn post_never_waits_for_a_full_mailbox() {
        let mailbox = <TokioEventQueue<u8> as Mailbox<u8>>::new(1);
        assert_eq!(mailbox.post(1), Ok(()));
        assert_eq!(mailbox.post(2), Err(PostError::MailboxFull(2)));
        assert_eq!(mailbox.recv().await, Ok(1));
    }

    #[tokio::test]
    async fn send_waits_for_capacity_and_closed_mailboxes_return_the_job() {
        use crate::EventQueue;

        let mailbox = std::sync::Arc::new(<TokioEventQueue<u8> as Mailbox<u8>>::new(1));
        mailbox.post(1).unwrap();
        let sender_mailbox = std::sync::Arc::clone(&mailbox);
        let sender = tokio::spawn(async move { sender_mailbox.send(2).await });
        tokio::task::yield_now().await;
        assert!(!sender.is_finished());
        assert_eq!(Mailbox::recv(mailbox.as_ref()).await, Ok(1));
        assert_eq!(sender.await.unwrap(), Ok(()));
        assert_eq!(Mailbox::recv(mailbox.as_ref()).await, Ok(2));

        EventQueue::close(mailbox.as_ref()).await;
        assert_eq!(mailbox.post(3), Err(PostError::ModuleUnavailable(3)));
    }
}

#[cfg(all(test, feature = "embassy"))]
mod embassy_tests {
    use super::Mailbox;
    use crate::{PostError, adapters::embassy::EmbassyEventQueue};

    #[test]
    fn embassy_mailbox_uses_static_capacity_and_nonblocking_post() {
        let mailbox = <EmbassyEventQueue<u8, 1> as Mailbox<u8>>::new(1);
        assert_eq!(mailbox.post(1), Ok(()));
        assert_eq!(mailbox.post(2), Err(PostError::MailboxFull(2)));
        assert_eq!(futures::executor::block_on(mailbox.recv()), Ok(1));
    }
}
