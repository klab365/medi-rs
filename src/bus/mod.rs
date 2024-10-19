mod bus_builder;

// -- flatten
pub use bus_builder::BusBuilder;

// -- use dependencies
use crate::error::{Error, Result};
use crate::handler_wrapper::HandlerWrapperTrait;
use crate::{FromResources, IntoCommand, IntoEvent, Resources, SharedHandler};
use std::any::TypeId;
use std::sync::Arc;

#[derive(Clone)]
pub struct Bus {
    req_handlers: SharedHandler<Arc<dyn HandlerWrapperTrait>>,
    evt_handlers: SharedHandler<Vec<Arc<dyn HandlerWrapperTrait>>>,
    resources: Resources,
}

impl FromResources for Bus {}

impl Bus {
    pub fn builder() -> BusBuilder {
        BusBuilder::default()
    }
}

impl Bus {
    pub(crate) fn new(
        resources: Resources,
        req_handlers: SharedHandler<Arc<dyn HandlerWrapperTrait>>,
        evt_handlers: SharedHandler<Vec<Arc<dyn HandlerWrapperTrait>>>,
    ) -> Self {
        let mut bus = Bus {
            req_handlers,
            evt_handlers,
            resources,
        };

        bus.resources.insert(bus.clone());
        bus
    }

    pub async fn send<Req, Res>(&self, req: Req) -> Result<Res>
    where
        Req: IntoCommand<Res> + Send + Sync + 'static,
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

    pub async fn publish<Evt>(&self, evt: Evt) -> Result<()>
    where
        Evt: IntoEvent + Send + Sync + 'static,
    {
        let type_id: TypeId = TypeId::of::<Evt>();
        let Some(handlers) = self.evt_handlers.get(&type_id).clone() else {
            return Err(Error::HandlerNotFound);
        };

        let evt = Box::new(evt.clone());
        for handler in handlers {
            let _ = handler.handle(self.resources.clone(), evt.clone()).await;
        }

        Ok(())
    }
}
