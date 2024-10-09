use super::{resources::Resources, AnyMap};
use std::any::TypeId;

#[derive(Default)]
pub struct ResourcesBuilder {
    map: Option<AnyMap>,
}

impl ResourcesBuilder {
    pub fn insert<T: Clone + Send + Sync + 'static>(&mut self, value: T) {
        self.map
            .get_or_insert_with(AnyMap::new)
            .insert(TypeId::of::<T>(), Box::new(value));
    }

    pub fn build(self) -> Resources {
        Resources::new(self.map)
    }
}
