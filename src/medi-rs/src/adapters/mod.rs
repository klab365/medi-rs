//! Runtime queue and task-spawn adapters for generated mediators.

pub mod queue;

#[cfg(feature = "embassy")]
pub mod embassy;
#[cfg(feature = "tokio")]
pub mod tokio;
#[cfg(feature = "wasm")]
pub mod wasm;

#[cfg(feature = "tokio")]
pub mod selected {
    pub use super::tokio::{TokioEventQueue as EventQueue, spawn};
}

#[cfg(all(feature = "wasm", not(feature = "tokio")))]
pub mod selected {
    pub use super::wasm::{WasmEventQueue as EventQueue, spawn};
}

#[cfg(all(feature = "embassy", not(feature = "tokio"), not(feature = "wasm")))]
pub mod selected {
    pub use super::embassy::EmbassyEventQueue as EventQueue;
}
