use thiserror::Error;

pub type Result<T> = std::result::Result<T, CodegenError>;

#[derive(Error, Debug)]
pub enum CodegenError {
    #[error("Template error: {0}")]
    Template(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid schema: {0}")]
    InvalidSchema(String),

    #[error("Code generation failed: {0}")]
    GenerationFailed(String),
}

impl From<tera::Error> for CodegenError {
    fn from(err: tera::Error) -> Self {
        CodegenError::Template(err.to_string())
    }
}
