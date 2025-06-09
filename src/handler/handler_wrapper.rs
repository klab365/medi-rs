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
            let type_name = std::any::type_name::<Req>();
            return Box::pin(async { Err(Error::CastError(type_name.to_string())) });
        };

        let handler = self.handler.clone();
        let fut = handler.handle(resources, *arg);
        Box::pin(async move {
            let res = fut.await?;
            Ok(Box::new(res) as Box<dyn Any + Send + Sync>)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::IntoCommand;

    use super::*;

    #[tokio::test]
    async fn test_cast_error_when_wrong_req_is_passed() {
        let handler = add_req_handler(test_handler);
        let resources = Resources::default();

        let wrong_req = Box::new(1);
        let res = handler.handle(resources, wrong_req).await;

        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), Error::CastError(_)));
    }

    struct BaseReq;
    impl IntoCommand<()> for BaseReq {}

    async fn test_handler(_req: BaseReq) -> Result<()> {
        Ok(())
    }

    pub fn add_req_handler<H, T, Req, Res>(h: H) -> Arc<dyn HandlerWrapperTrait + Send + Sync + 'static>
    where
        H: Handler<T, Req, Res> + Sync + Send + 'static,
        T: Sync + Send + 'static,
        Req: IntoCommand<Res> + Sync + Send + 'static,
        Res: Sync + Send + 'static,
    {
        h.into_dyn()
    }
}
