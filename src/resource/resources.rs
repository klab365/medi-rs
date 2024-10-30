use std::{any::TypeId, sync::Arc};

use super::AnyMap;

#[derive(Debug, Clone, Default)]
pub struct Resources {
    map: Option<AnyMap>,
}

impl Resources {
    pub fn get<T: Clone + Send + Sync + 'static>(&self) -> Option<T> {
        let map = self.map.as_ref()?;
        let res = map.get(&TypeId::of::<T>());
        let res = res?;
        let res = res.downcast_ref::<T>();
        let res = res?;
        Some(res.clone())
    }

    pub(crate) fn insert<T: Clone + Send + Sync + 'static>(&mut self, value: T) {
        self.map
            .get_or_insert(AnyMap::new())
            .insert(TypeId::of::<T>(), Arc::new(value));
    }
}
