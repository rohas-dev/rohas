use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandlerContext {
    pub handler_name: String,

    pub payload: serde_json::Value,

    pub query_params: HashMap<String, String>,

    pub metadata: HashMap<String, String>,

    pub timestamp: String,
}

impl HandlerContext {
    pub fn new(handler_name: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            handler_name: handler_name.into(),
            payload,
            query_params: HashMap::new(),
            metadata: HashMap::new(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    pub fn with_query_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.query_params.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggeredEvent {
    pub event_name: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandlerResult {
    pub success: bool,

    pub data: Option<serde_json::Value>,

    pub error: Option<String>,

    pub execution_time_ms: u64,

    #[serde(default)]
    pub triggers: Vec<TriggeredEvent>,

    #[serde(default)]
    pub auto_trigger_payloads: std::collections::HashMap<String, serde_json::Value>,
}

impl HandlerResult {
    pub fn success(data: serde_json::Value, execution_time_ms: u64) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            execution_time_ms,
            triggers: Vec::new(),
            auto_trigger_payloads: std::collections::HashMap::new(),
        }
    }

    pub fn error(error: impl Into<String>, execution_time_ms: u64) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error.into()),
            execution_time_ms,
            triggers: Vec::new(),
            auto_trigger_payloads: std::collections::HashMap::new(),
        }
    }

    pub fn with_trigger(mut self, event_name: impl Into<String>, payload: serde_json::Value) -> Self {
        self.triggers.push(TriggeredEvent {
            event_name: event_name.into(),
            payload,
        });
        self
    }

    pub fn with_auto_trigger_payload(mut self, event_name: impl Into<String>, payload: serde_json::Value) -> Self {
        self.auto_trigger_payloads.insert(event_name.into(), payload);
        self
    }
}

#[async_trait::async_trait]
pub trait Handler: Send + Sync {
    async fn execute(&self, context: HandlerContext) -> crate::Result<HandlerResult>;

    fn name(&self) -> &str;
}
