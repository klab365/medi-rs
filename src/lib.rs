mod bus;
mod error;
mod handler;
mod resource;

// flatten the module structure
pub use error::*;
pub use bus::*;
pub use handler::*;
pub use resource::*;
use uuid::Uuid;

/// IntoCommand trait will be used to mark command or query types for the bus
pub trait IntoCommand<Res>
where
    Self: Send + Sync + 'static,
{
}

/// IntoEvent trait will be used to mark event types for the bus
/// Each event should have an unique id
pub trait IntoEvent: Clone
where
    Self: Send + Sync + 'static,
{
    /// Get the id of the event
    /// Each event should have an unique id
    fn get_id(&self) -> Uuid {
        Uuid::new_v4()
    }
}

// Implement the handler traits
// The maximum of resource parameter will be 7
impl_handler!();
impl_handler!(T1);
impl_handler!(T1, T2);
impl_handler!(T1, T2, T3);
impl_handler!(T1, T2, T3, T4);
