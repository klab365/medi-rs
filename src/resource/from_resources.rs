use super::error::{ResourceError, Result};
use super::resources::{self, Resources};

pub trait FromResources {
    fn from_resources(resources: &Resources) -> Result<Self>
    where
        Self: Sized + Clone + Send + Sync + 'static,
    {
        resources.get::<Self>().ok_or_else(|| ResourceError::ResourceNotFound)
    }
}

impl<T> FromResources for Option<T>
where
    T: FromResources,
    T: Sized + Clone + Send + Sync + 'static,
{
    fn from_resources(resources: &Resources) -> Result<Self>
    where
        Self: Sized + Clone + Send + Sync + 'static,
    {
        Ok(T::from_resources(resources).ok())
    }
}
