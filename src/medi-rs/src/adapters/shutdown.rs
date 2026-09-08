//! Cooperative cancellation primitive injected into runtime tasks.

use core::future::{Future, poll_fn};
use core::sync::atomic::{AtomicBool, Ordering};
use core::task::Poll;
use futures::task::AtomicWaker;

/// A cooperative shutdown notification for a generated `#[medi_task]`.
///
/// Declare it as `signal: &ShutdownSignal` immediately after the optional
/// `&Mediator` parameter. `shutdown()` cancels the signal before awaiting the
/// task. Each task receives its own wait slot; all slots are cancelled by the
/// same mediator shutdown operation.
pub struct ShutdownSignal {
    cancelled: AtomicBool,
    waker: AtomicWaker,
}

impl ShutdownSignal {
    /// Create an active signal.
    pub const fn new() -> Self {
        Self {
            cancelled: AtomicBool::new(false),
            waker: AtomicWaker::new(),
        }
    }

    /// Return whether mediator shutdown has begun.
    pub fn is_shutdown(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    /// Wait until mediator shutdown begins.
    pub fn cancelled(&self) -> impl Future<Output = ()> + '_ {
        poll_fn(|cx| {
            if self.is_shutdown() {
                return Poll::Ready(());
            }
            self.waker.register(cx.waker());
            if self.is_shutdown() {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })
    }

    #[doc(hidden)]
    pub fn cancel(&self) {
        if !self.cancelled.swap(true, Ordering::AcqRel) {
            self.waker.wake();
        }
    }
}

impl Default for ShutdownSignal {
    fn default() -> Self {
        Self::new()
    }
}
