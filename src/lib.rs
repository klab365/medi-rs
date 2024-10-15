mod bus;
mod error;
mod handler;
mod resource;

// flatten the module structure
pub use self::error::{Error, Result};
pub use bus::*;
pub use handler::*;
pub use resource::*;

// types of request..
pub trait IntoReq<Res>
where
    Self: Send + Sync + 'static,
{
}

// Implement the handler traits
// The maximum of resource parameter will be 7
impl_handler!(IntoReq, Resources,);
impl_handler!(IntoReq, Resources, T1);
impl_handler!(IntoReq, Resources, T1, T2);
impl_handler!(IntoReq, Resources, T1, T2, T3);
impl_handler!(IntoReq, Resources, T1, T2, T3, T4);
impl_handler!(IntoReq, Resources, T1, T2, T3, T4, T5);
impl_handler!(IntoReq, Resources, T1, T2, T3, T4, T5, T6);
impl_handler!(IntoReq, Resources, T1, T2, T3, T4, T5, T6, T7);
