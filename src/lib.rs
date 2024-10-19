mod bus;
mod error;
mod event;
mod handler;
mod resource;

// flatten the module structure
pub use self::error::{Error, Result};
pub use bus::*;
pub use event::*;
pub use handler::*;
pub use resource::*;

/// IntoCommand trait will be used to mark command or query types for the bus
pub trait IntoCommand<Res>
where
    Self: Send + Sync + 'static,
{
}

// Implement the handler traits
// The maximum of resource parameter will be 7
impl_handler!();
impl_handler!(T1);
impl_handler!(T1, T2);
impl_handler!(T1, T2, T3);
impl_handler!(T1, T2, T3, T4);
