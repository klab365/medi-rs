//! Typed actor replies, including an allocation-free Embassy response pool.

#[cfg(any(feature = "tokio", feature = "wasm"))]
mod implementation {
    /// Sender half of a typed actor reply channel.
    pub type ReplySender<T> = futures::channel::oneshot::Sender<T>;
    /// Receiver half of a typed actor reply channel.
    pub type ReplyReceiver<T> = futures::channel::oneshot::Receiver<T>;

    /// Create a typed, single-use actor reply channel.
    pub fn reply_channel<T>() -> (ReplySender<T>, ReplyReceiver<T>) {
        futures::channel::oneshot::channel()
    }
}

#[cfg(feature = "embassy")]
mod implementation {
    use core::sync::atomic::{AtomicUsize, Ordering};
    use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
    use embassy_sync::signal::Signal;

    const FREE: usize = 0;
    const RESERVED: usize = 1;
    const CANCELED: usize = 2;
    const COMPLETE: usize = 3;
    const STATUS_MASK: usize = 3;

    struct Slot<T> {
        state: AtomicUsize,
        signal: Signal<CriticalSectionRawMutex, (usize, T)>,
    }

    impl<T> Slot<T> {
        fn new() -> Self {
            Self {
                state: AtomicUsize::new(FREE),
                signal: Signal::new(),
            }
        }
    }

    /// Key carried by a mailbox job to identify the caller's reply slot.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct ReplyKey {
        slot: usize,
        generation: usize,
    }

    /// Fixed-capacity reply storage for heapless actor `send` operations.
    pub struct ReplyPool<T, const CAPACITY: usize> {
        slots: [Slot<T>; CAPACITY],
    }

    impl<T, const CAPACITY: usize> ReplyPool<T, CAPACITY> {
        /// Create an empty response pool.
        pub fn new() -> Self {
            assert!(CAPACITY > 0, "reply pool capacity must be greater than zero");
            Self {
                slots: core::array::from_fn(|_| Slot::new()),
            }
        }

        /// Reserve one response slot. Returns `None` when all requests are in flight.
        pub fn reserve(&self) -> Option<ReplyTicket<'_, T, CAPACITY>> {
            for (index, slot) in self.slots.iter().enumerate() {
                let mut state = slot.state.load(Ordering::Acquire);
                loop {
                    if state & STATUS_MASK != FREE {
                        break;
                    }
                    let generation = (state & !STATUS_MASK).wrapping_add(STATUS_MASK + 1);
                    let reserved = generation | RESERVED;
                    match slot
                        .state
                        .compare_exchange_weak(state, reserved, Ordering::AcqRel, Ordering::Acquire)
                    {
                        Ok(_) => {
                            slot.signal.reset();
                            return Some(ReplyTicket {
                                pool: self,
                                key: ReplyKey {
                                    slot: index,
                                    generation,
                                },
                                armed: true,
                            });
                        }
                        Err(actual) => state = actual,
                    }
                }
            }
            None
        }

        /// Complete a request. Late replies to canceled tickets are discarded.
        pub fn respond(&self, key: ReplyKey, value: T) {
            let Some(slot) = self.slots.get(key.slot) else {
                return;
            };
            let state = slot.state.load(Ordering::Acquire);
            if state & !STATUS_MASK != key.generation {
                return;
            }
            match state & STATUS_MASK {
                RESERVED => {
                    slot.signal.signal((key.generation, value));
                    if slot
                        .state
                        .compare_exchange(
                            key.generation | RESERVED,
                            key.generation | COMPLETE,
                            Ordering::AcqRel,
                            Ordering::Acquire,
                        )
                        .is_err()
                    {
                        let canceled = key.generation | CANCELED;
                        slot.signal.reset();
                        let _ = slot.state.compare_exchange(
                            canceled,
                            key.generation | FREE,
                            Ordering::AcqRel,
                            Ordering::Acquire,
                        );
                    }
                }
                CANCELED => {
                    let _ = slot.state.compare_exchange(
                        key.generation | CANCELED,
                        key.generation | FREE,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    );
                }
                _ => {}
            }
        }
    }

    impl<T, const CAPACITY: usize> Default for ReplyPool<T, CAPACITY> {
        fn default() -> Self {
            Self::new()
        }
    }

    /// Caller-owned reservation. Dropping it cancels the reply slot.
    pub struct ReplyTicket<'a, T, const CAPACITY: usize> {
        pool: &'a ReplyPool<T, CAPACITY>,
        key: ReplyKey,
        armed: bool,
    }

    impl<T, const CAPACITY: usize> ReplyTicket<'_, T, CAPACITY> {
        /// Key to attach to the corresponding actor mailbox request.
        pub fn key(&self) -> ReplyKey {
            self.key
        }

        /// Release a reservation when its mailbox request was not enqueued.
        pub fn cancel_unsent(mut self) {
            let slot = &self.pool.slots[self.key.slot];
            let _ = slot.state.compare_exchange(
                self.key.generation | RESERVED,
                self.key.generation | FREE,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
            self.armed = false;
        }

        /// Wait for the actor's typed reply.
        pub async fn wait(mut self) -> T {
            let slot = &self.pool.slots[self.key.slot];
            loop {
                let (generation, value) = slot.signal.wait().await;
                if generation == self.key.generation {
                    self.release();
                    return value;
                }
            }
        }

        fn release(&mut self) {
            if !self.armed {
                return;
            }
            let slot = &self.pool.slots[self.key.slot];
            loop {
                let state = slot.state.load(Ordering::Acquire);
                if state & !STATUS_MASK != self.key.generation {
                    break;
                }
                let status = state & STATUS_MASK;
                if status == FREE {
                    break;
                }
                if status == COMPLETE {
                    slot.signal.reset();
                }
                let target_status = if status == RESERVED { CANCELED } else { FREE };
                if slot
                    .state
                    .compare_exchange(
                        state,
                        self.key.generation | target_status,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    )
                    .is_ok()
                {
                    break;
                }
            }
            self.armed = false;
        }
    }

    impl<T, const CAPACITY: usize> Drop for ReplyTicket<'_, T, CAPACITY> {
        fn drop(&mut self) {
            self.release();
        }
    }

    #[cfg(test)]
    mod tests {
        use super::ReplyPool;
        use futures::executor::block_on;

        #[test]
        fn reserves_and_routes_replies_without_allocating() {
            let pool = ReplyPool::<u8, 1>::new();
            let ticket = pool.reserve().expect("one slot should be available");
            let key = ticket.key();
            assert!(pool.reserve().is_none());
            pool.respond(key, 42);
            assert_eq!(block_on(ticket.wait()), 42);
            assert!(pool.reserve().is_some());
        }

        #[test]
        fn canceled_request_releases_slot_when_actor_replies() {
            let pool = ReplyPool::<u8, 1>::new();
            let ticket = pool.reserve().expect("one slot should be available");
            let key = ticket.key();
            drop(ticket);
            assert!(pool.reserve().is_none());
            pool.respond(key, 42);
            assert!(pool.reserve().is_some());
        }

        #[test]
        fn unsent_request_releases_slot_immediately() {
            let pool = ReplyPool::<u8, 1>::new();
            pool.reserve().expect("one slot should be available").cancel_unsent();
            assert!(pool.reserve().is_some());
        }
    }
}

pub use implementation::*;

#[cfg(any(feature = "tokio", feature = "wasm"))]
#[cfg(test)]
mod tests {
    use super::reply_channel;
    use futures::executor::block_on;

    #[test]
    fn typed_reply_channel_delivers_value_once() {
        let (sender, receiver) = reply_channel();
        sender.send(42_u8).expect("reply receiver is alive");
        assert_eq!(block_on(receiver), Ok(42));
    }
}
