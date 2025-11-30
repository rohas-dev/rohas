use thiserror::Error;

pub type Result<T> = std::result::Result<T, TelemetryError>;

#[derive(Error, Debug)]
pub enum TelemetryError {
    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Storage backend error: {0}")]
    StorageBackend(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid key format: {0}")]
    InvalidKey(String),

    #[error("Not found: {0}")]
    NotFound(String),
}

