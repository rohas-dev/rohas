use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggeredEventInfo {
    pub event_name: String,
    pub timestamp: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceStep {
    pub name: String,
    pub handler_name: String,
    pub duration_ms: u64,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub triggered_events: Vec<TriggeredEventInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceRecord {
    pub id: String,
    pub entry_point: String,
    pub entry_type: TraceEntryType,
    pub status: TraceStatus,
    pub duration_ms: u64,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub steps: Vec<TraceStep>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TraceEntryType {
    Api,
    Event,
    Cron,
    WebSocket,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TraceStatus {
    Success,
    Failed,
    Running,
}

