use std::{any::TypeId, collections::HashMap};

use serde::{de::DeserializeOwned, Serialize};

use crate::traits::{Handler, HandlerWrapperTrait, IntoReq, SharedHandler};

use super::Bus;

pub struct BusBuilder {
    req_handlers: SharedHandler<Box<dyn HandlerWrapperTrait>>,
}

impl Default for BusBuilder {
    fn default() -> Self {
        BusBuilder {
            req_handlers: HashMap::new(),
        }
    }
}

impl BusBuilder {
    pub fn add_req_handler<H, Req, Res>(mut self, h: H) -> Self
    where
        H: Handler<Req, Res> + Sync + Send + 'static,
        Req: IntoReq<Res> + Sync + Send + 'static,
        Res: Serialize + DeserializeOwned + Sync + Send + 'static,
    {
        let type_id = TypeId::of::<Req>();

        if self.req_handlers.contains_key(&type_id) {
            let type_name = std::any::type_name::<Req>();
            panic!("Route already exists for type: {}", type_name);
        }

        self.req_handlers.insert(type_id, h.into_dyn());

        self
    }

    pub fn build(self) -> Bus {
        Bus::new(self.req_handlers)
    }
}
