#[macro_export]
macro_rules! impl_static_handler {
    ($($T:ident : $I:ident),*) => {
        impl<F, Fut, R, Req, Res, E, $($T, $I,)*>
            $crate::StaticHandler<R, Req, ($($crate::Dependency<$T, $I>,)*)> for F
        where
            F: FnOnce($($T,)* Req) -> Fut + Clone + Send,
            Req: Send,
            Res: Send,
            E: Send,
            $(R: $crate::tlist::Get<$T, $I>,)*
            Fut: core::future::Future<Output = core::result::Result<Res, E>> + Send,
        {
            type Response = Res;
            type Error = E;

            fn handle(self, resources: &R, value: Req)
                -> impl core::future::Future<Output = core::result::Result<Self::Response, Self::Error>> + Send
            {
                let _ = resources;
                let future = self($($crate::tlist::get::<$T, $I, R>(resources),)* value);
                future
            }
        }
    };
}
