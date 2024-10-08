use crate::{common, error::Result};
use std::marker::PhantomData;
use serde::{de::DeserializeOwned, Serialize};

pub trait IntoReq<Res>: Serialize + DeserializeOwned + Send {}

#[async_trait::async_trait]
pub trait HandlerWrapperTrait: Send + Sync {
    async fn handle(&self, value: &common::Decoded) -> Result<common::Encoded>;
}

#[async_trait::async_trait]
pub trait Handler<Req, Res>: Clone
where
    Req: IntoReq<Res> + Send + Sync + 'static,
    Res: Serialize + DeserializeOwned + Send + Sync + 'static,
{
    async fn handle(self, value: Req) -> Result<Res>;

    fn into_dyn(self) -> Box<dyn HandlerWrapperTrait>
    where
        Self: Sized + Send + Sync + 'static,
    {
        Box::new(HandlerWrapper::new(self))
    }
}

struct HandlerWrapper<H, Req, Res> {
    handler: H,
    _phantom: PhantomData<(Req, Res)>,
}

impl<H, Req, Res> HandlerWrapper<H, Req, Res> {
    fn new(handler: H) -> Self {
        HandlerWrapper {
            handler,
            _phantom: PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<H, Req, Res> HandlerWrapperTrait for HandlerWrapper<H, Req, Res>
where
    H: Handler<Req, Res> + Sync + Send + 'static,
    Req: IntoReq<Res> + Sync + Send + 'static,
    Res: Serialize + DeserializeOwned + Sync + Send + 'static,
{
    async fn handle(&self, value: &[u8]) -> Result<Vec<u8>> {
        let arg1 = common::deserialize(value)?;
        let handler = self.handler.clone();
        let res = handler.handle(arg1).await?;
        let res = bincode::serialize(&res).unwrap();
        Ok(res)
    }
}

#[async_trait::async_trait]
impl<Req, Res, F> Handler<Req, Res> for F
where
    F: FnOnce(Req) -> Result<Res> + Clone + Send + Sync + 'static,
    Req: IntoReq<Res> + Sync + Send + 'static,
    Res: Serialize + DeserializeOwned + Sync + Send + 'static,
{
    async fn handle(self, value: Req) -> Result<Res> {
        let arg1 = value;
        let res = (self)(arg1);
        res
    }
}
