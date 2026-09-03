use core::future::Future;

/// Type-level dependency declaration for a [`StaticHandler`].
#[doc(hidden)]
pub struct Dependency<T, I>(core::marker::PhantomData<fn() -> (T, I)>);

/// A statically dispatched async handler function.
pub trait StaticHandler<R, Req, Dependencies>: Clone {
    type Response;
    type Error;

    fn handle(
        self,
        resources: &R,
        value: Req,
    ) -> impl Future<Output = core::result::Result<Self::Response, Self::Error>> + Send;
}

#[cfg(test)]
mod tests {
    use super::StaticHandler;
    use crate::{Dependency, tlist::Here};
    use alloc::{format, string::String};

    #[derive(Clone)]
    struct Prefix(&'static str);
    struct Greet(&'static str);
    #[derive(Debug, Eq, PartialEq)]
    struct HandlerError;

    async fn greet(prefix: Prefix, command: Greet) -> Result<String, HandlerError> {
        Ok(format!("{} {}", prefix.0, command.0))
    }

    async fn invoke<F, R, Req, Dependencies>(handler: F, resources: &R, request: Req) -> Result<F::Response, F::Error>
    where
        F: StaticHandler<R, Req, Dependencies>,
    {
        handler.handle(resources, request).await
    }

    #[tokio::test]
    async fn invokes_a_handler_with_a_typed_resource() {
        let resources = (Prefix("Hello"), ());
        assert_eq!(
            invoke::<_, _, _, (Dependency<Prefix, Here>,)>(greet, &resources, Greet("Ada"))
                .await
                .unwrap(),
            "Hello Ada"
        );
    }
}
