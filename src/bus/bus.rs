use crate::bus::bus_builder::BusBuilder;
use crate::common;
use crate::error::{Error, Result};
use crate::traits::{HandlerWrapperTrait, IntoReq, SharedHandler};
use serde::de::DeserializeOwned;
use std::any::TypeId;

pub struct Bus {
    req_handlers: SharedHandler<Box<dyn HandlerWrapperTrait>>,
}

impl Bus {
    pub fn builder() -> BusBuilder {
        BusBuilder::default()
    }
}

impl Bus {
    pub(crate) fn new(req_handlers: SharedHandler<Box<dyn HandlerWrapperTrait>>) -> Self {
        Bus { req_handlers }
    }

    pub async fn send<Req, Res>(&self, req: Req) -> Result<Res>
    where
        Req: IntoReq<Res> + Send + Sync + 'static,
        Res: DeserializeOwned + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<Req>();
        let handler = self.req_handlers.get(&type_id);
        let Some(handler) = handler else {
            return Err(Error::HandlerNotFound);
        };

        let req = Box::new(req);
        let res = handler.handle(req).await?;
        let res = common::deserialize(&res)?;
        Ok(res)
    }
}
