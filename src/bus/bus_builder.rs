use crate::{handler_wrapper::HandlerWrapperTrait, FromResources, Handler, IntoCommand, IntoEvent, SharedHandler};
use crate::{HandlerResult, Resources};
use std::any::TypeId;
use std::sync::Arc;

use super::Bus;

#[derive(Default)]
pub struct BusBuilder {
    req_handlers: SharedHandler<Arc<dyn HandlerWrapperTrait>>,
    evt_handlers: SharedHandler<Vec<Arc<dyn HandlerWrapperTrait>>>,
    resources: Resources,
}

impl BusBuilder {
    pub fn add_req_handler<H, T, Req, Res>(mut self, h: H) -> Self
    where
        H: Handler<T, Req, Res> + Sync + Send + 'static,
        T: Sync + Send + 'static,
        Req: IntoCommand<Res> + Sync + Send + 'static,
        Res: Sync + Send + 'static,
    {
        let type_id = TypeId::of::<Req>();

        if self.req_handlers.contains_key(&type_id) {
            let type_name = std::any::type_name::<Req>();
            panic!("Route already exists for type: {}", type_name);
        }

        self.req_handlers.insert(type_id, h.into_dyn());

        self
    }

    pub fn add_event_handler<H, T, Evt>(mut self, h: H) -> Self
    where
        H: Handler<T, Evt, ()> + Sync + Send + 'static,
        T: Sync + Send + 'static,
        Evt: IntoEvent + Sync + Send + 'static,
    {
        let type_id = TypeId::of::<Evt>();

        self.evt_handlers.entry(type_id).or_default();
        let handlers = self.evt_handlers.get_mut(&type_id).unwrap();
        handlers.push(h.into_dyn());

        self
    }

    pub fn append_resources<T>(mut self, value: T) -> Self
    where
        T: FromResources + Clone + Send + Sync + 'static,
    {
        self.resources.insert(value);
        self
    }

    pub fn build(self) -> HandlerResult<Bus> {
        let bus = Bus::new(self.resources, self.req_handlers, self.evt_handlers);

        Ok(bus)
    }
}
