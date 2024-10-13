use crate::{handler_wrapper::HandlerWrapperTrait, FromResources, Handler, IntoReq, ResourcesBuilder, SharedHandler};
use std::any::TypeId;

use super::Bus;

#[derive(Default)]
pub struct BusBuilder {
    req_handlers: SharedHandler<Box<dyn HandlerWrapperTrait>>,
    resource_builder: ResourcesBuilder,
}

impl BusBuilder {
    pub fn add_req_handler<H, T, Req, Res>(mut self, h: H) -> Self
    where
        H: Handler<T, Req, Res> + Sync + Send + 'static,
        T: Sync + Send + 'static,
        Req: IntoReq<Res> + Sync + Send + 'static,
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

    pub fn append_resources<T>(mut self, value: T) -> Self
    where
        T: FromResources + Clone + Send + Sync + 'static,
    {
        self.resource_builder.insert(value);
        self
    }

    pub fn build(self) -> Bus {
        Bus::new(self.resource_builder.build(), self.req_handlers)
    }
}
