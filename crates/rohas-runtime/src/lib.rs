pub mod error;
pub mod executor;
pub mod handler;
pub mod node_runtime;
pub mod python_runtime;
pub mod rust_runtime;

pub use error::{Result, RuntimeError};
pub use executor::Executor;
pub use handler::{Handler, HandlerContext, HandlerResult};
pub use rust_runtime::RustRuntime;

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub language: Language,
    pub project_root: std::path::PathBuf,
    pub timeout_seconds: u64,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            language: Language::TypeScript,
            project_root: std::env::current_dir().unwrap_or_default(),
            timeout_seconds: 30,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    TypeScript,
    Python,
    Rust,
}

impl Language {
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::TypeScript => "typescript",
            Language::Python => "python",
            Language::Rust => "rust",
        }
    }

    pub fn file_extension(&self) -> &'static str {
        match self {
            Language::TypeScript => "ts",
            Language::Python => "py",
            Language::Rust => "rs",
        }
    }
}
