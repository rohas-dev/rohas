use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Database connection error: {0}")]
    Connection(String),

    #[error("Query execution error: {0}")]
    Query(String),

    #[error("Model validation error: {0}")]
    Validation(String),

    #[error("Migration error: {0}")]
    Migration(String),

    #[error("Code generation error: {0}")]
    Codegen(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("SQLx error: {0}")]
    Sqlx(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Other error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        Error::Sqlx(err.to_string())
    }
}

