use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

pub type HandlerResult<T> = core::result::Result<T, HandlerError>;

#[derive(Debug)]
pub struct HandlerError {
    holder: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl HandlerError {
    pub(crate) fn new<T>(value: T) -> Self
    where
        T: Any + Send + Sync,
    {
        let mut holder = HashMap::<TypeId, Box<dyn Any + Send + Sync>>::with_capacity(1);
        let type_id = TypeId::of::<T>();
        holder.insert(type_id, Box::new(value));
        HandlerError { holder }
    }

    pub fn get<T: Any + Send + Sync>(&self) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        self.holder.get(&type_id).and_then(|v| v.downcast_ref::<T>())
    }
}

pub trait IntoHandlerError
where
    Self: Sized + Send + Sync + 'static,
{
    fn into_handler_error(self) -> HandlerError {
        HandlerError::new(self)
    }
}

impl core::fmt::Display for HandlerError {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

impl std::error::Error for HandlerError {}

// -- implementation for IntoHandlerError
impl IntoHandlerError for HandlerError {
    fn into_handler_error(self) -> HandlerError {
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::HandlerError;

    #[test]
    fn test_handler_error() {
        let error = HandlerError::new("error");
        assert_eq!(error.get::<&str>(), Some(&"error"));
        assert_eq!(error.get::<i32>(), None);
    }

    #[allow(clippy::useless_conversion)]
    #[test]
    fn into_handler_error_should_return_handler_error_when_handler_error() {
        let err: HandlerError = HandlerError::new("error").into();
        assert_eq!(err.get::<&str>(), Some(&"error"));
    }
}
