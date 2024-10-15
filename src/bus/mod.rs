mod bus_builder;

// -- flatten
pub use bus_builder::BusBuilder;

use crate::error::{Error, Result};
use crate::handler_wrapper::HandlerWrapperTrait;
use crate::{IntoReq, Resources, SharedHandler};
use std::any::TypeId;

pub struct Bus {
    req_handlers: SharedHandler<Box<dyn HandlerWrapperTrait>>,
    resources: Resources,
}

impl Bus {
    pub fn builder() -> BusBuilder {
        BusBuilder::default()
    }
}

impl Bus {
    pub(crate) fn new(resources: Resources, req_handlers: SharedHandler<Box<dyn HandlerWrapperTrait>>) -> Self {
        Bus {
            resources,
            req_handlers,
        }
    }

    pub async fn send<Req, Res>(&self, req: Req) -> Result<Res>
    where
        Req: IntoReq<Res> + Send + Sync + 'static,
        Res: Send + Sync + 'static,
    {
        let type_id = TypeId::of::<Req>();

        let handler = self.req_handlers.get(&type_id);
        let Some(handler) = handler else {
            return Err(Error::HandlerNotFound);
        };

        let req = Box::new(req);
        let res = handler.handle(self.resources.clone(), req).await?;

        let Ok(res) = res.downcast::<Res>() else {
            return Err(Error::CastError);
        };

        Ok(*res)
    }
}
