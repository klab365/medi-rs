use super::resources::Resources;
use crate::error::{Error, HandlerResult};

pub trait FromResources {
    fn from_resources(resources: &Resources) -> HandlerResult<Self>
    where
        Self: Sized + Clone + Send + Sync + 'static,
    {
        resources.get::<Self>().ok_or_else(|| Error::ResourceNotFound)
    }
}
