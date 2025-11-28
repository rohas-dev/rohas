use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

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

pub struct TraceStore {
    traces: Arc<RwLock<Vec<TraceRecord>>>,
    max_traces: usize,
}

impl TraceStore {
    pub fn new(max_traces: usize) -> Self {
        Self {
            traces: Arc::new(RwLock::new(Vec::new())),
            max_traces,
        }
    }

    pub async fn start_trace(
        &self,
        entry_point: String,
        entry_type: TraceEntryType,
        metadata: HashMap<String, String>,
    ) -> String {
        let id = Uuid::new_v4().to_string();
        let started_at = Utc::now().to_rfc3339();

        let trace = TraceRecord {
            id: id.clone(),
            entry_point,
            entry_type,
            status: TraceStatus::Running,
            duration_ms: 0,
            started_at,
            completed_at: None,
            steps: Vec::new(),
            error: None,
            metadata,
        };

        let mut traces = self.traces.write().await;
        traces.push(trace);
        
        if traces.len() > self.max_traces {
            traces.remove(0);
        }

        id
    }

    pub async fn add_step(
        &self,
        trace_id: &str,
        handler_name: String,
        duration_ms: u64,
        success: bool,
        error: Option<String>,
    ) {
        self.add_step_with_triggers(trace_id, handler_name, duration_ms, success, error, Vec::new()).await;
    }

    pub async fn add_step_with_triggers(
        &self,
        trace_id: &str,
        handler_name: String,
        duration_ms: u64,
        success: bool,
        error: Option<String>,
        triggered_events: Vec<TriggeredEventInfo>,
    ) {
        let mut traces = self.traces.write().await;
        if let Some(trace) = traces.iter_mut().find(|t| t.id == trace_id) {
            trace.steps.push(TraceStep {
                name: handler_name.clone(),
                handler_name,
                duration_ms,
                success,
                error,
                timestamp: Utc::now().to_rfc3339(),
                triggered_events,
            });
        }
    }

    pub async fn complete_trace(
        &self,
        trace_id: &str,
        status: TraceStatus,
        error: Option<String>,
    ) {
        let mut traces = self.traces.write().await;
        if let Some(trace) = traces.iter_mut().find(|t| t.id == trace_id) {
            trace.status = status;
            trace.error = error;
            trace.completed_at = Some(Utc::now().to_rfc3339());
            
            if let Ok(started) = DateTime::parse_from_rfc3339(&trace.started_at) {
                if let Some(completed_str) = trace.completed_at.as_ref() {
                    if let Ok(completed) = DateTime::parse_from_rfc3339(completed_str) {
                        let duration = completed.signed_duration_since(started);
                        trace.duration_ms = duration.num_milliseconds() as u64;
                    } else {
                        trace.duration_ms = trace.steps.iter().map(|s| s.duration_ms).sum();
                    }
                } else {
                    trace.duration_ms = trace.steps.iter().map(|s| s.duration_ms).sum();
                }
            }
        }
    }

    pub async fn get_traces(&self, limit: Option<usize>) -> Vec<TraceRecord> {
        let traces = self.traces.read().await;
        let mut result: Vec<TraceRecord> = traces.iter().rev().cloned().collect();
        
        if let Some(limit) = limit {
            result.truncate(limit);
        }
        
        result
    }

    pub async fn get_traces_since(&self, since_id: Option<&str>, timeout_secs: u64) -> Vec<TraceRecord> {
        use tokio::time::{sleep, Duration};
        
        let start_time = std::time::Instant::now();
        let timeout = Duration::from_secs(timeout_secs);
        let check_interval = Duration::from_millis(500);
        
        loop {
            let traces = self.traces.read().await;
            let mut result: Vec<TraceRecord> = traces.iter().rev().cloned().collect();
            drop(traces);
            
            if let Some(since) = since_id {
                if let Some(since_index) = result.iter().position(|t| t.id == since) {
                    result = result[..since_index].to_vec();
                } else {
                    result = result;
                }
            }
            
            if !result.is_empty() {
                return result;
            }
            
            if start_time.elapsed() >= timeout {
                return vec![];
            }
            
            sleep(check_interval).await;
        }
    }

    pub async fn clear(&self) {
        let mut traces = self.traces.write().await;
        traces.clear();
    }
}

