use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub project_root: PathBuf,

    pub language: Language,

    pub server: ServerConfig,

    pub adapter: AdapterConfig,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            project_root: std::env::current_dir().unwrap_or_default(),
            language: Language::TypeScript,
            server: ServerConfig::default(),
            adapter: AdapterConfig::default(),
        }
    }
}

impl EngineConfig {
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)?;
        let toml_config: TomlConfig = toml::from_str(&content)?;

        Ok(toml_config.into_engine_config()?)
    }

    pub fn from_project_root() -> anyhow::Result<Self> {
        let project_root = std::env::current_dir()?;
        let config_path = project_root.join("config").join("rohas.toml");

        if !config_path.exists() {
            anyhow::bail!("Config file not found: {}", config_path.display());
        }

        Self::from_file(&config_path)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Language {
    TypeScript,
    Python,
}

impl From<Language> for rohas_runtime::Language {
    fn from(lang: Language) -> Self {
        match lang {
            Language::TypeScript => rohas_runtime::Language::TypeScript,
            Language::Python => rohas_runtime::Language::Python,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub enable_cors: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
            enable_cors: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterConfig {
    pub adapter_type: AdapterType,
    pub buffer_size: usize,
}

impl Default for AdapterConfig {
    fn default() -> Self {
        Self {
            adapter_type: AdapterType::Memory,
            buffer_size: 1000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdapterType {
    Memory,
    Nats { url: String },
    Kafka { brokers: String },
    RabbitMQ { url: String },
    Sqs,
}

#[derive(Debug, Deserialize)]
struct TomlConfig {
    project: TomlProject,
    server: TomlServer,
    adapter: TomlAdapter,
}

#[derive(Debug, Deserialize)]
struct TomlProject {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    version: String,
    language: String,
}

#[derive(Debug, Deserialize)]
struct TomlServer {
    host: String,
    port: u16,
    enable_cors: bool,
}

#[derive(Debug, Deserialize)]
struct TomlAdapter {
    #[serde(rename = "type")]
    adapter_type: String,
    buffer_size: usize,
}

impl TomlConfig {
    fn into_engine_config(self) -> anyhow::Result<EngineConfig> {
        let language = match self.project.language.to_lowercase().as_str() {
            "typescript" | "ts" => Language::TypeScript,
            "python" | "py" => Language::Python,
            _ => anyhow::bail!("Unsupported language: {}", self.project.language),
        };

        let adapter_type = match self.adapter.adapter_type.to_lowercase().as_str() {
            "memory" => AdapterType::Memory,
            _ => anyhow::bail!("Unsupported adapter type: {}", self.adapter.adapter_type),
        };

        Ok(EngineConfig {
            project_root: std::env::current_dir()?,
            language,
            server: ServerConfig {
                host: self.server.host,
                port: self.server.port,
                enable_cors: self.server.enable_cors,
            },
            adapter: AdapterConfig {
                adapter_type,
                buffer_size: self.adapter.buffer_size,
            },
        })
    }
}
