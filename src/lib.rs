mod bus;
mod error;
mod event;
mod handler;
mod resource;

// flatten the module structure
pub use bus::*;
pub use error::*;
pub use handler::*;
pub use resource::*;

/// IntoCommand trait will be used to mark command or query types for the bus
pub trait IntoCommand<Res>
where
    Self: Send + Sync + 'static,
{
}

/// IntoEvent trait will be used to mark event types for the bus
/// Each event should have an unique id
pub trait IntoEvent
where
    Self: Send + Sync + 'static,
{
}

//-- region: Implement the handler traits
impl_handler!();
impl_handler!(T1);
impl_handler!(T1, T2);
impl_handler!(T1, T2, T3);
impl_handler!(T1, T2, T3, T4);
impl_handler!(T1, T2, T3, T4, T5);
impl_handler!(T1, T2, T3, T4, T5, T6);
impl_handler!(T1, T2, T3, T4, T5, T6, T7);
//-- endregion: Implement the handler traits
