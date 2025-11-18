use thiserror::Error;

pub type Result<T> = std::result::Result<T, ParseError>;

#[derive(Error, Debug, Clone)]
pub enum ParseError {
    #[error("Syntax error at line {line}, column {column}: {message}")]
    SyntaxError {
        line: usize,
        column: usize,
        message: String,
    },

    #[error("Invalid model definition: {0}")]
    InvalidModel(String),

    #[error("Invalid API definition: {0}")]
    InvalidApi(String),

    #[error("Invalid event definition: {0}")]
    InvalidEvent(String),

    #[error("Invalid cron definition: {0}")]
    InvalidCron(String),

    #[error("Invalid type: {0}")]
    InvalidType(String),

    #[error("Invalid attribute: {0}")]
    InvalidAttribute(String),

    #[error("Duplicate definition: {0}")]
    DuplicateDefinition(String),

    #[error("Undefined reference: {0}")]
    UndefinedReference(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Parse error: {0}")]
    ParseError(String),
}

impl From<std::io::Error> for ParseError {
    fn from(err: std::io::Error) -> Self {
        ParseError::IoError(err.to_string())
    }
}

impl From<pest::error::Error<crate::grammar::Rule>> for ParseError {
    fn from(err: pest::error::Error<crate::grammar::Rule>) -> Self {
        let (line, column) = match err.line_col {
            pest::error::LineColLocation::Pos((line, col)) => (line, col),
            pest::error::LineColLocation::Span((line, col), _) => (line, col),
        };
        ParseError::SyntaxError {
            line,
            column,
            message: err.variant.to_string(),
        }
    }
}
