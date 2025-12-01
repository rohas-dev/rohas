pub mod error;
pub mod executor;
pub mod handler;
pub mod node_rpc_runtime;
pub mod node_runtime;
pub mod python_runtime;

pub use error::{Result, RuntimeError};
pub use executor::Executor;
pub use handler::{Handler, HandlerContext, HandlerResult};

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub language: Language,
    pub project_root: std::path::PathBuf,
    pub timeout_seconds: u64,
    pub node_execution_mode: NodeExecutionMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeExecutionMode {
    Embedded,
    Rpc,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            language: Language::TypeScript,
            project_root: std::env::current_dir().unwrap_or_default(),
            timeout_seconds: 30,
            node_execution_mode: NodeExecutionMode::Rpc,
        }
    }
}

impl Default for NodeExecutionMode {
    fn default() -> Self {
        NodeExecutionMode::Rpc
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    TypeScript,
    Python,
}

impl Language {
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::TypeScript => "typescript",
            Language::Python => "python",
        }
    }

    pub fn file_extension(&self) -> &'static str {
        match self {
            Language::TypeScript => "ts",
            Language::Python => "py",
        }
    }
}
