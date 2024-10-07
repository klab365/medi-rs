use crate::traits::Result;
use crate::traits::{Handler, HandlerWrapperTrait, IntoReq};
use serde::{de::DeserializeOwned, Serialize};
use std::{any::TypeId, collections::HashMap};

pub struct Bus {
    req_handlers: HashMap<TypeId, Box<dyn HandlerWrapperTrait>>,
}

impl Bus {
    pub fn new() -> Self {
        Bus {
            req_handlers: HashMap::new(),
        }
    }

    pub fn add_req_handler<H, Req, Res>(&mut self, h: H)
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
    }

    pub async fn send<Req, Res>(&self, req: Req) -> Result<Res>
    where
        Req: IntoReq<Res> + 'static,
        Res: DeserializeOwned + 'static,
    {
        let type_id = TypeId::of::<Req>();
        let handler = self.req_handlers.get(&type_id).unwrap();

        let encoded = req.into_encoded();
        let res = handler.handle(&encoded).await?;
        let res = bincode::deserialize(&res).unwrap();
        Ok(res)
    }
}
