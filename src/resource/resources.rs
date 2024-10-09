use std::{any::TypeId, sync::Arc};

use super::{resources_builder::ResourcesBuilder, AnyMap};



pub struct Resources {
    map: Arc<Option<AnyMap>>
}

impl Resources {
    pub fn builder() -> ResourcesBuilder {
        ResourcesBuilder::default()
    }
}

impl Resources {
    pub fn new(map: Option<AnyMap>) -> Self {
        Self {
            map: Arc::new(map)
        }
    }

    pub fn get<T: Clone + Send + Sync + 'static>(&self) -> Option<T> {
        let map = self.map.as_ref();
        let Some(map) = map else {
            panic!("Resources not initialized");
        };

        map.get(&TypeId::of::<T>()).map(|v| v.downcast_ref::<T>().unwrap().clone())
    }
}
