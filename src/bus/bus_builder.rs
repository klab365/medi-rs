use std::{any::TypeId, collections::HashMap};

use crate::{
    resource::{from_resources::FromResources, resources::Resources, resources_builder::ResourcesBuilder},
    traits::{Handler, HandlerWrapperTrait, IntoReq, SharedHandler},
};

use super::Bus;

pub struct BusBuilder {
    req_handlers: SharedHandler<Box<dyn HandlerWrapperTrait>>,
    resource_builder: ResourcesBuilder,
}

impl Default for BusBuilder {
    fn default() -> Self {
        BusBuilder {
            req_handlers: HashMap::new(),
            resource_builder: ResourcesBuilder::default(),
        }
    }
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
