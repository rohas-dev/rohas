use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, AdapterError>;

#[derive(Error, Debug)]
pub enum AdapterError {
    #[error("AWS SQS error: {0}")]
    AwsSqs(String),

    #[error("AWS EventBridge error: {0}")]
    AwsEventBridge(String),

    #[error("Queue not found: {0}")]
    QueueNotFound(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid message format: {0}")]
    InvalidMessage(String),

    #[error("Configuration error: {0}")]
    Configuration(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub topic: String,
    pub payload: serde_json::Value,
    pub timestamp: String,
    pub metadata: HashMap<String, String>,
}

impl Message {
    pub fn new(topic: impl Into<String>, payload: serde_json::Value) -> Self {
        use std::time::SystemTime;
        Self {
            topic: topic.into(),
            payload,
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .to_string(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

#[async_trait]
pub trait MessageHandler: Send + Sync {
    async fn handle(&self, message: Message) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct AwsConfig {
    pub region: String,
    pub queue_prefix: Option<String>, // For SQS
    pub event_bus_name: Option<String>, // For EventBridge (default: "default")
    pub source: Option<String>, // For EventBridge (default: "rohas")
    pub visibility_timeout_seconds: Option<i32>, // For SQS
    pub message_retention_seconds: Option<i32>, // For SQS
    pub receive_wait_time_seconds: Option<i32>, // For SQS (long polling)
}

impl Default for AwsConfig {
    fn default() -> Self {
        Self {
            region: "us-east-1".to_string(),
            queue_prefix: Some("rohas-".to_string()),
            event_bus_name: None, // Use default event bus
            source: Some("rohas".to_string()),
            visibility_timeout_seconds: Some(30),
            message_retention_seconds: Some(345600), // 4 days
            receive_wait_time_seconds: Some(20), // Long polling
        }
    }
}

