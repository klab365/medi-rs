use crate::{
    error::{Error, Result},
    resource::{from_resources::FromResources, resources::Resources},
};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    marker::PhantomData,
};

pub type SharedHandler<T> = HashMap<TypeId, T>;

pub trait IntoReq<Res>: Send {}

#[async_trait::async_trait]
pub trait HandlerWrapperTrait: Send + Sync {
    async fn handle(
        &self,
        resources: Resources,
        value: Box<dyn Any + Send + Sync>,
    ) -> Result<Box<dyn Any + Send + Sync>>;
}

#[async_trait::async_trait]
pub trait Handler<T, Req, Res>: Clone
where
    T: Send + Sync + 'static,
    Req: IntoReq<Res> + Send + Sync + 'static,
    Res: Send + Sync + 'static,
{
    async fn handle(self, resources: Resources, value: Req) -> Result<Res>;

    fn into_dyn(self) -> Box<dyn HandlerWrapperTrait>
    where
        Self: Sized + Send + Sync + 'static,
    {
        Box::new(HandlerWrapper::new(self))
    }
}

struct HandlerWrapper<H, T, Req, Res> {
    handler: H,
    _phantom: PhantomData<(T, Req, Res)>,
}

impl<H, T, Req, Res> HandlerWrapper<H, T, Req, Res> {
    fn new(handler: H) -> Self {
        HandlerWrapper {
            handler,
            _phantom: PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<H, T, Req, Res> HandlerWrapperTrait for HandlerWrapper<H, T, Req, Res>
where
    H: Handler<T, Req, Res> + Sync + Send + 'static,
    T: Send + Sync + 'static,
    Req: IntoReq<Res> + Sync + Send + 'static,
    Res: Sync + Send + 'static,
{
    async fn handle(
        &self,
        resources: Resources,
        value: Box<dyn Any + Send + Sync>,
    ) -> Result<Box<dyn Any + Send + Sync>> {
        let Ok(arg) = value.downcast::<Req>() else {
            return Err(Error::CastError);
        };

        let handler = self.handler.clone();
        let res = handler.handle(resources, *arg).await?;
        let res = Box::new(res);
        Ok(res)
    }
}

// implementation handler...

// without resources but with one request
#[async_trait::async_trait]
impl<Req, Res, F> Handler<(), Req, Res> for F
where
    F: FnOnce(Req) -> Result<Res> + Clone + Send + Sync + 'static,
    Req: IntoReq<Res> + Sync + Send + 'static,
    Res: Sync + Send + 'static,
{
    async fn handle(self, _resources: Resources, value: Req) -> Result<Res> {
        let arg1 = value;
        let res = (self)(arg1);
        res
    }
}

// with one resource and one request
#[async_trait::async_trait]
#[async_trait::async_trait]
impl<T1, Req, Res, F> Handler<T1, Req, Res> for F
where
    F: FnOnce(T1, Req) -> Result<Res> + Clone + Send + Sync + 'static,
    Req: IntoReq<Res> + Sync + Send + 'static,
    Res: Sync + Send + 'static,
    T1: FromResources + Clone + Send + Sync + 'static,
{
    async fn handle(self, resources: Resources, value: Req) -> Result<Res> {
        let arg1 = value;
        let res1 = T1::from_resources(&resources)?;
        let res = (self)(res1, arg1);
        res
    }
}
