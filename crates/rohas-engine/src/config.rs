use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub project_root: PathBuf,

    pub language: Language,

    pub server: ServerConfig,

    pub adapter: AdapterConfig,

    pub telemetry: TelemetryConfig,

    pub workbench: WorkbenchConfig,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            project_root: std::env::current_dir().unwrap_or_default(),
            language: Language::TypeScript,
            server: ServerConfig::default(),
            adapter: AdapterConfig::default(),
            telemetry: TelemetryConfig::default(),
            workbench: WorkbenchConfig::default(),
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
    Aws {
        region: String,
        #[serde(rename = "aws_type")]
        aws_type: String, // "sqs" or "eventbridge"
        queue_prefix: Option<String>, // For SQS
        event_bus_name: Option<String>, // For EventBridge
        source: Option<String>, // For EventBridge
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    #[serde(rename = "type")]
    pub adapter_type: TelemetryAdapterType,
    
    #[serde(default = "default_telemetry_path")]
    pub path: String,
    
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,
    
    #[serde(default = "default_max_cache_size")]
    pub max_cache_size: usize,
    
    #[serde(default = "default_true")]
    pub enable_metrics: bool,
    
    #[serde(default = "default_true")]
    pub enable_logs: bool,
    
    #[serde(default = "default_true")]
    pub enable_traces: bool,
}

fn default_telemetry_path() -> String {
    ".rohas/telemetry".to_string()
}

fn default_retention_days() -> u32 {
    30
}

fn default_max_cache_size() -> usize {
    1000
}

fn default_true() -> bool {
    true
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            adapter_type: TelemetryAdapterType::RocksDB,
            path: default_telemetry_path(),
            retention_days: default_retention_days(),
            max_cache_size: default_max_cache_size(),
            enable_metrics: default_true(),
            enable_logs: default_true(),
            enable_traces: default_true(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TelemetryAdapterType {
    RocksDB,
    Prometheus,
    InfluxDB,
    TimescaleDB,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkbenchConfig {
    pub api_key: String,
    #[serde(default)]
    pub allowed_origins: Vec<String>,
}

impl Default for WorkbenchConfig {
    fn default() -> Self {
        Self {
            api_key: generate_api_key(),
            allowed_origins: Vec::new(),
        }
    }
}

fn generate_api_key() -> String {
    let bytes = Uuid::new_v4().into_bytes();
    general_purpose::STANDARD.encode(bytes)
}

#[derive(Debug, Deserialize)]
struct TomlConfig {
    project: TomlProject,
    server: TomlServer,
    adapter: TomlAdapter,
    #[serde(default)]
    telemetry: Option<TomlTelemetry>,
    #[serde(default)]
    workbench: Option<TomlWorkbench>,
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
    // AWS-specific fields
    region: Option<String>,
    #[serde(rename = "aws_type")]
    aws_type: Option<String>, // "sqs" or "eventbridge"
    queue_prefix: Option<String>, // For SQS
    event_bus_name: Option<String>, // For EventBridge
    source: Option<String>, // For EventBridge
}

#[derive(Debug, Deserialize)]
struct TomlTelemetry {
    #[serde(rename = "type")]
    adapter_type: Option<String>,
    path: Option<String>,
    retention_days: Option<u32>,
    max_cache_size: Option<usize>,
    enable_metrics: Option<bool>,
    enable_logs: Option<bool>,
    enable_traces: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct TomlWorkbench {
    api_key: Option<String>,
    allowed_origins: Option<Vec<String>>,
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
            "aws" => {
                let aws_type = self.adapter.aws_type.as_deref()
                    .unwrap_or("sqs")
                    .to_lowercase();
                if aws_type != "sqs" && aws_type != "eventbridge" {
                    anyhow::bail!("Unsupported AWS adapter type: {}. Must be 'sqs' or 'eventbridge'", aws_type);
                }
                AdapterType::Aws {
                    region: self.adapter.region.unwrap_or_else(|| "us-east-1".to_string()),
                    aws_type,
                    queue_prefix: self.adapter.queue_prefix,
                    event_bus_name: self.adapter.event_bus_name,
                    source: self.adapter.source,
                }
            }
            "sqs" => AdapterType::Aws {
                region: self.adapter.region.unwrap_or_else(|| "us-east-1".to_string()),
                aws_type: "sqs".to_string(),
                queue_prefix: self.adapter.queue_prefix,
                event_bus_name: None,
                source: None,
            },
            _ => anyhow::bail!("Unsupported adapter type: {}", self.adapter.adapter_type),
        };

        let telemetry = if let Some(telemetry) = self.telemetry {
            let adapter_type = match telemetry.adapter_type.as_deref().unwrap_or("rocksdb").to_lowercase().as_str() {
                "rocksdb" => TelemetryAdapterType::RocksDB,
                "prometheus" => TelemetryAdapterType::Prometheus,
                "influxdb" => TelemetryAdapterType::InfluxDB,
                "timescaledb" => TelemetryAdapterType::TimescaleDB,
                _ => anyhow::bail!("Unsupported telemetry adapter type: {}", telemetry.adapter_type.unwrap_or_default()),
            };
            
            TelemetryConfig {
                adapter_type,
                path: telemetry.path.unwrap_or_else(default_telemetry_path),
                retention_days: telemetry.retention_days.unwrap_or_else(default_retention_days),
                max_cache_size: telemetry.max_cache_size.unwrap_or_else(default_max_cache_size),
                enable_metrics: telemetry.enable_metrics.unwrap_or_else(default_true),
                enable_logs: telemetry.enable_logs.unwrap_or_else(default_true),
                enable_traces: telemetry.enable_traces.unwrap_or_else(default_true),
            }
        } else {
            TelemetryConfig::default()
        };

        let workbench = if let Some(workbench) = self.workbench {
            WorkbenchConfig {
                api_key: workbench.api_key.unwrap_or_else(generate_api_key),
                allowed_origins: workbench.allowed_origins.unwrap_or_default(),
            }
        } else {
            WorkbenchConfig::default()
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
            telemetry,
            workbench,
        })
    }
}
