#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Handler not found")]
    HandlerNotFound,
}

pub type Result<T> = std::result::Result<T, Error>;
