// mod handler_error;
pub mod handler_wrapper;
mod macros;

// --flatten
// pub use handler_error::*;
use handler_wrapper::HandlerWrapper;
use handler_wrapper::HandlerWrapperTrait;

use crate::Resources;
use crate::Result;
use std::sync::Arc;
use std::{any::TypeId, collections::HashMap};

pub type SharedHandler<T> = HashMap<TypeId, T>;

pub trait Handler<T, Req, Res>: Clone
where
    T: Send + Sync + 'static,
    Req: Send + Sync + 'static,
    Res: Send + Sync + 'static,
{
    type Future: std::future::Future<Output = Result<Res>> + Send + Sync + 'static;

    fn handle(self, resources: Resources, value: Req) -> Self::Future;

    #[allow(private_interfaces)]
    fn into_dyn(self) -> Arc<dyn HandlerWrapperTrait>
    where
        Self: Sized + Send + Sync + 'static,
    {
        Arc::new(HandlerWrapper::new(self)) as Arc<dyn HandlerWrapperTrait>
    }
}
