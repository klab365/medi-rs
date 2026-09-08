//! Completion tracking for generated mediator workers and runtime tasks.

use core::future::{Future, poll_fn};
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use core::task::Poll;
use futures::task::AtomicWaker;

/// Tracks work started by a generated mediator.
///
/// Generated code registers every event worker and `#[medi_task]` before it is
/// spawned, and marks it complete when its future returns. [`Self::wait`] is
/// therefore the completion guarantee used by mediator shutdown.
pub struct Lifecycle {
    running: AtomicUsize,
    shutdown_requested: AtomicBool,
    waker: AtomicWaker,
}

impl Lifecycle {
    /// Create an idle lifecycle tracker.
    pub const fn new() -> Self {
        Self {
            running: AtomicUsize::new(0),
            shutdown_requested: AtomicBool::new(false),
            waker: AtomicWaker::new(),
        }
    }

    /// Request shutdown, returning `true` only for the caller that initiated it.
    pub fn request_shutdown(&self) -> bool {
        self.shutdown_requested
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    /// Register work immediately before spawning it.
    pub fn begin(&self) {
        self.running.fetch_add(1, Ordering::AcqRel);
    }

    /// Mark previously registered work as complete.
    pub fn finish(&self) {
        if self.running.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.waker.wake();
        }
    }

    /// Wait until all registered workers and tasks have returned.
    pub fn wait(&self) -> impl Future<Output = ()> + '_ {
        poll_fn(|cx| {
            if self.running.load(Ordering::Acquire) == 0 {
                return Poll::Ready(());
            }
            self.waker.register(cx.waker());
            if self.running.load(Ordering::Acquire) == 0 {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })
    }
}

impl Default for Lifecycle {
    fn default() -> Self {
        Self::new()
    }
}
