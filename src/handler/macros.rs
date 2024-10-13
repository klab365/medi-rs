#[macro_export]
macro_rules! impl_handler {
    ($req_trait:ident, $K:ty, $($T:ident), *) => {
        impl<F, Fut, $($T,)* Req, Res, E> Handler<($($T,)*), Req, Res> for F
        where
            F: FnOnce($($T,)* Req) -> Fut + Clone + Send + Sync + 'static,
            Req: $req_trait<Res>,
            Res: Sync + Send + 'static,
            $($T: FromResources + Clone + Send + Sync + 'static,)*
            E: IntoHandlerError,
            Fut: std::future::Future<Output = core::result::Result<Res, E>> + Send + Sync + 'static,
        {
            type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Res>> + Send + Sync>>;

            #[allow(unused)]
            fn handle(self, resources: Resources, value: Req) -> Self::Future {
                Box::pin(async move {
                    let arg = value;
                    let res = self($($T::from_resources(&resources)?,)* arg).await;

                    match res {
                        Ok(res) => Ok(res),
                        Err(e) => {
                            let he = IntoHandlerError::into_handler_error(e);
                            Err(he.into())
                        }
                    }
                })
            }
        }
    };
}