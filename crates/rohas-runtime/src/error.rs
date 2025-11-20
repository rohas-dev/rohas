use thiserror::Error;

pub type Result<T> = std::result::Result<T, RuntimeError>;

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("Handler execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Handler not found: {0}")]
    HandlerNotFound(String),

    #[error("Timeout: handler exceeded {0} seconds")]
    Timeout(u64),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Python error: {0}")]
    PythonError(String),

    #[error("Node.js error: {0}")]
    NodeError(String),

    #[error("Invalid handler response: {0}")]
    InvalidResponse(String),
}

// Implement conversion from pyo3::PyErr
impl From<pyo3::PyErr> for RuntimeError {
    fn from(err: pyo3::PyErr) -> Self {
        RuntimeError::PythonError(err.to_string())
    }
}
