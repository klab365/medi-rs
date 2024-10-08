#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Handler not found")]
    HandlerNotFound,

    #[error("Serialization error")]
    SerializationError,
}

pub type Result<T> = std::result::Result<T, Error>;
