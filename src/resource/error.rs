

pub type Result<T> = std::result::Result<T, ResourceError>;

#[derive(Debug, thiserror::Error)]
pub enum ResourceError {
    #[error("Resource not found")]
    ResourceNotFound,
}
