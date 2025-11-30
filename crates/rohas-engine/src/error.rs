use thiserror::Error;

pub type Result<T> = std::result::Result<T, EngineError>;

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Parser error: {0}")]
    Parser(#[from] rohas_parser::ParseError),

    #[error("Runtime error: {0}")]
    Runtime(#[from] rohas_runtime::RuntimeError),

    #[error("Cron error: {0}")]
    Cron(#[from] rohas_cron::CronError),

    #[error("Adapter error: {0}")]
    Adapter(#[from] adapter_memory::AdapterError),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("Event dispatch error: {0}")]
    EventDispatch(String),

    #[error("Engine not initialized")]
    NotInitialized,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Initialization error: {0}")]
    Initialization(String),
}
