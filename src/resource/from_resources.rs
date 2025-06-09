use super::resources::Resources;
use crate::error::{Error, Result};

pub trait FromResources {
    fn from_resources(resources: &Resources) -> Result<Self>
    where
        Self: Sized + Clone + Send + Sync + 'static,
    {
        resources.get::<Self>().ok_or_else(|| Error::ResourceNotFound)
    }
}
