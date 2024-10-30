use std::any::{Any, TypeId};

use crate::IntoEvent;

pub(crate) struct EventWrapper<Evt> {
    type_id: TypeId,
    event: Evt,
}

impl<Evt> EventWrapper<Evt>
where
    Evt: IntoEvent + Send + Sync + 'static,
{
    pub(crate) fn new(event: Evt) -> Self {
        EventWrapper {
            type_id: TypeId::of::<Evt>(),
            event,
        }
    }
}

pub(crate) trait EventWrapperTrait: Send + Sync {
    fn into_dyn(self) -> Box<dyn EventWrapperTrait + Send + Sync + 'static>;

    fn get_type_id(&self) -> TypeId;

    fn get_any(&self) -> Box<dyn Any + Send + Sync>;
}

impl<Evt> EventWrapperTrait for EventWrapper<Evt>
where
    Evt: IntoEvent + Clone + Send + Sync + 'static,
{
    fn into_dyn(self) -> Box<dyn EventWrapperTrait + Send + Sync + 'static> {
        Box::new(self)
    }

    fn get_any(&self) -> Box<dyn Any + Send + Sync> {
        Box::new(self.event.clone())
    }

    fn get_type_id(&self) -> TypeId {
        self.type_id
    }
}
