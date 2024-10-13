use std::{any::TypeId, sync::Arc};

use super::AnyMap;

#[derive(Debug, Clone, Default)]
pub struct Resources {
    map: Arc<Option<AnyMap>>,
}

impl Resources {
    pub(crate) fn new(map: Option<AnyMap>) -> Self {
        Self { map: Arc::new(map) }
    }

    pub fn get<T: Clone + Send + Sync + 'static>(&self) -> Option<T> {
        let map = self.map.as_ref();
        let Some(map) = map else {
            return None;
        };

        let res = map.get(&TypeId::of::<T>());
        let res = res?;

        let res = res.downcast_ref::<T>();
        let res = res?;

        Some(res.clone())
    }
}
