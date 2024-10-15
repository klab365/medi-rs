use std::{any::Any, marker::PhantomData, pin::Pin};

use crate::Error;
use crate::Resources;
use crate::Result;

use super::Handler;

pub(crate) struct HandlerWrapper<H, T, Req, Res> {
    handler: H,
    _phantom: PhantomData<(T, Req, Res)>,
}

impl<H, T, Req, Res> HandlerWrapper<H, T, Req, Res> {
    pub(crate) fn new(handler: H) -> Self {
        HandlerWrapper {
            handler,
            _phantom: PhantomData,
        }
    }
}

#[allow(clippy::type_complexity)]
pub(crate) trait HandlerWrapperTrait: Send + Sync {
    fn handle(
        &self,
        resources: Resources,
        value: Box<dyn Any + Send + Sync>,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Box<dyn Any + Send + Sync>>> + Send + Sync>>;
}

impl<H, TResource, Req, Res> HandlerWrapperTrait for HandlerWrapper<H, TResource, Req, Res>
where
    H: Handler<TResource, Req, Res> + Sync + Send + 'static,
    TResource: Send + Sync + 'static,
    Req: Sync + Send + 'static,
    Res: Send + Sync + 'static,
{
    fn handle(
        &self,
        resources: Resources,
        value: Box<dyn Any + Send + Sync>,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Box<dyn Any + Send + Sync>>> + Send + Sync>> {
        let Ok(arg) = value.downcast::<Req>() else {
            return Box::pin(async { Err(Error::CastError) });
        };

        let handler = self.handler.clone();
        let fut = handler.handle(resources, *arg);
        Box::pin(async move {
            let res = fut.await?;
            Ok(Box::new(res) as Box<dyn Any + Send + Sync>)
        })
    }
}
