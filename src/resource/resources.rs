use std::{any::TypeId, sync::Arc};

use super::AnyMap;

#[derive(Debug, Clone, Default)]
pub struct Resources {
    map: Option<AnyMap>,
}

impl Resources {
    pub fn get<T: Clone + Send + Sync + 'static>(&self) -> Option<T> {
        self.map.as_ref()?.get(&TypeId::of::<T>())?.downcast_ref::<T>().cloned()
    }

    pub(crate) fn insert<T: Clone + Send + Sync + 'static>(&mut self, value: T) {
        self.map
            .get_or_insert_with(AnyMap::new)
            .insert(TypeId::of::<T>(), Arc::new(value));
    }
}
