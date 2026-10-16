//! One-time ownership transfer for generated module actors.
//!
//! A mediator owns module values until it starts. Each generated actor worker
//! then takes its module from an [`ActorSlot`] and owns it for the rest of its
//! lifetime, so handler calls can use `&mut self` without a mutex guard.

#[cfg(any(feature = "tokio", feature = "wasm"))]
mod implementation {
    /// Storage that transfers a module to exactly one actor worker.
    pub struct ActorSlot<T>(std::sync::Mutex<Option<T>>);

    impl<T> ActorSlot<T> {
        /// Create a slot containing a module that has not started yet.
        pub fn new(value: T) -> Self {
            Self(std::sync::Mutex::new(Some(value)))
        }

        /// Take the module for its actor worker. Subsequent calls return `None`.
        pub fn take(&self) -> Option<T> {
            self.0.lock().ok()?.take()
        }
    }
}

#[cfg(feature = "embassy")]
mod implementation {
    use core::cell::RefCell;
    use embassy_sync::blocking_mutex::Mutex;
    use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;

    /// Storage that transfers a module to exactly one actor worker.
    pub struct ActorSlot<T>(Mutex<CriticalSectionRawMutex, RefCell<Option<T>>>);

    impl<T> ActorSlot<T> {
        /// Create a slot containing a module that has not started yet.
        pub fn new(value: T) -> Self {
            Self(Mutex::new(RefCell::new(Some(value))))
        }

        /// Take the module for its actor worker. Subsequent calls return `None`.
        pub fn take(&self) -> Option<T> {
            self.0.lock(|value| value.borrow_mut().take())
        }
    }
}

pub use implementation::ActorSlot;

#[cfg(test)]
mod tests {
    use super::ActorSlot;

    #[test]
    fn transfers_ownership_once() {
        let slot = ActorSlot::new("button");
        assert_eq!(slot.take(), Some("button"));
        assert_eq!(slot.take(), None);
    }
}
