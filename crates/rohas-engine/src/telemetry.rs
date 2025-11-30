use adapter_rocksdb::RocksDBAdapter;
use rohas_telemetry::{LogStore, MetricStore, TelemetryAdapter, TraceStore as TelemetryTraceStore, traces::{TraceStep as TelemetryTraceStep, TriggeredEventInfo as TelemetryTriggeredEventInfo}, storage::IterateCallback};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::Utc;
use std::collections::HashMap;

use crate::trace::{TraceEntryType, TraceRecord, TraceStatus, TraceStep, TriggeredEventInfo};

pub struct TelemetryManager {
    _adapter: TelemetryAdapter,
    trace_store: Arc<TelemetryTraceStore>,
    log_store: Arc<LogStore>,
    metric_store: Arc<MetricStore>,
    active_traces: Arc<RwLock<HashMap<String, TraceRecord>>>,
    retention_days: u32,
}

impl TelemetryManager {
    pub async fn new(telemetry_path: PathBuf, retention_days: u32) -> Result<Self, Box<dyn std::error::Error>> {
        let rocksdb_adapter = RocksDBAdapter::new(telemetry_path).await?;
        let storage: Arc<dyn rohas_telemetry::StorageAdapter> = Arc::new(rocksdb_adapter);
        
        let trace_store = Arc::new(TelemetryTraceStore::new(storage.clone()));
        let log_store = Arc::new(LogStore::new(storage.clone()));
        let metric_store = Arc::new(MetricStore::new(storage.clone()));
        
        let storage_for_adapter: Box<dyn rohas_telemetry::StorageAdapter> = {
            Box::new(StorageWrapper(storage.clone()))
        };
        let telemetry_adapter = TelemetryAdapter::new(storage_for_adapter);
        
        Ok(Self {
            _adapter: telemetry_adapter,
            trace_store,
            log_store,
            metric_store,
            active_traces: Arc::new(RwLock::new(HashMap::new())),
            retention_days,
        })
    }

    pub fn retention_days(&self) -> u32 {
        self.retention_days
    }

    pub async fn cleanup_old_traces(&self) -> Result<usize, Box<dyn std::error::Error>> {
        if self.retention_days == 0 {
            return Ok(0);
        }

        let cutoff_time = Utc::now() - chrono::Duration::days(self.retention_days as i64);
        self.trace_store.delete_older_than(cutoff_time).await
            .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())) as Box<dyn std::error::Error>)
    }

    pub fn trace_store(&self) -> Arc<TelemetryTraceStore> {
        self.trace_store.clone()
    }

    pub fn log_store(&self) -> Arc<LogStore> {
        self.log_store.clone()
    }

    pub fn metric_store(&self) -> Arc<MetricStore> {
        self.metric_store.clone()
    }
}

struct StorageWrapper(Arc<dyn rohas_telemetry::StorageAdapter>);

#[async_trait::async_trait]
impl rohas_telemetry::StorageAdapter for StorageWrapper {
    async fn put(&self, key: &[u8], value: &[u8]) -> rohas_telemetry::Result<()> {
        self.0.put(key, value).await
    }

    async fn get(&self, key: &[u8]) -> rohas_telemetry::Result<Option<Vec<u8>>> {
        self.0.get(key).await
    }

    async fn delete(&self, key: &[u8]) -> rohas_telemetry::Result<()> {
        self.0.delete(key).await
    }

    async fn get_by_prefix(&self, prefix: &[u8]) -> rohas_telemetry::Result<Vec<Vec<u8>>> {
        self.0.get_by_prefix(prefix).await
    }

    async fn iterate(&self, prefix: &[u8], callback: Box<dyn rohas_telemetry::storage::IterateCallback>) -> rohas_telemetry::Result<()> {
        self.0.iterate(prefix, callback).await
    }
}

pub struct TraceStore {
    telemetry: Arc<TelemetryManager>,
    active_traces: Arc<RwLock<HashMap<String, TraceRecord>>>,
}

impl TraceStore {
    pub fn new(telemetry: Arc<TelemetryManager>) -> Self {
        Self {
            active_traces: telemetry.active_traces.clone(),
            telemetry,
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

        let mut active = self.active_traces.write().await;
        active.insert(id.clone(), trace);

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
        let mut active = self.active_traces.write().await;
        if let Some(trace) = active.get_mut(trace_id) {
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
        let mut active = self.active_traces.write().await;
        if let Some(mut trace) = active.remove(trace_id) {
            trace.status = status;
            trace.error = error;
            trace.completed_at = Some(Utc::now().to_rfc3339());
            
            if let Ok(started) = chrono::DateTime::parse_from_rfc3339(&trace.started_at) {
                if let Some(completed_str) = trace.completed_at.as_ref() {
                    if let Ok(completed) = chrono::DateTime::parse_from_rfc3339(completed_str) {
                        let duration = completed.signed_duration_since(started);
                        trace.duration_ms = duration.num_milliseconds() as u64;
                    } else {
                        trace.duration_ms = trace.steps.iter().map(|s| s.duration_ms).sum();
                    }
                } else {
                    trace.duration_ms = trace.steps.iter().map(|s| s.duration_ms).sum();
                }
            }

            let telemetry_entry = rohas_telemetry::TraceEntry {
                id: trace.id.clone(),
                entry_point: trace.entry_point.clone(),
                entry_type: format!("{:?}", trace.entry_type).to_lowercase(),
                status: format!("{:?}", trace.status).to_lowercase(),
                duration_ms: trace.duration_ms,
                started_at: trace.started_at.clone(),
                completed_at: trace.completed_at.clone(),
                steps: trace.steps.iter().map(|s| TelemetryTraceStep {
                    name: s.name.clone(),
                    handler_name: s.handler_name.clone(),
                    duration_ms: s.duration_ms,
                    success: s.success,
                    error: s.error.clone(),
                    timestamp: s.timestamp.clone(),
                    triggered_events: s.triggered_events.iter().map(|e| TelemetryTriggeredEventInfo {
                        event_name: e.event_name.clone(),
                        timestamp: e.timestamp.clone(),
                        duration_ms: e.duration_ms,
                    }).collect(),
                }).collect(),
                error: trace.error.clone(),
                metadata: trace.metadata.clone(),
            };

            let _ = self.telemetry.trace_store().store(telemetry_entry).await;
        }
    }

    fn convert_telemetry_entry(e: rohas_telemetry::TraceEntry) -> TraceRecord {
        TraceRecord {
            id: e.id,
            entry_point: e.entry_point,
            entry_type: match e.entry_type.as_str() {
                "api" => TraceEntryType::Api,
                "event" => TraceEntryType::Event,
                "cron" => TraceEntryType::Cron,
                "websocket" => TraceEntryType::WebSocket,
                _ => TraceEntryType::Api,
            },
            status: match e.status.as_str() {
                "success" => TraceStatus::Success,
                "failed" => TraceStatus::Failed,
                "running" => TraceStatus::Running,
                _ => TraceStatus::Running,
            },
            duration_ms: e.duration_ms,
            started_at: e.started_at,
            completed_at: e.completed_at,
            steps: e.steps.into_iter().map(|s| TraceStep {
                name: s.name,
                handler_name: s.handler_name,
                duration_ms: s.duration_ms,
                success: s.success,
                error: s.error,
                timestamp: s.timestamp,
                triggered_events: s.triggered_events.into_iter().map(|e| TriggeredEventInfo {
                    event_name: e.event_name,
                    timestamp: e.timestamp,
                    duration_ms: e.duration_ms,
                }).collect(),
            }).collect(),
            error: e.error,
            metadata: e.metadata,
        }
    }

    pub async fn get_traces(&self, limit: Option<usize>) -> Vec<TraceRecord> {
        let active = self.active_traces.read().await;
        let mut active_traces: Vec<TraceRecord> = active.values().cloned().collect();
        drop(active);
        
        let query_limit = limit.unwrap_or(usize::MAX); 
        let mut telemetry_traces = if self.telemetry.retention_days() == 0 {
            match self.telemetry.trace_store().get_all(limit).await {
                Ok(entries) => {
                    entries.into_iter().map(Self::convert_telemetry_entry).collect()
                }
                Err(_) => Vec::new(),
            }
        } else {
            let end_time = Utc::now();
            let start_time = end_time - chrono::Duration::days(self.telemetry.retention_days() as i64);
            match self.telemetry.trace_store().query_range(start_time, end_time, limit).await {
                Ok(entries) => {
                    entries.into_iter().map(Self::convert_telemetry_entry).collect()
                }
                Err(_) => Vec::new(),
            }
        };
        
        let mut all_traces: std::collections::HashMap<String, TraceRecord> = std::collections::HashMap::new();
        
        for trace in telemetry_traces {
            all_traces.insert(trace.id.clone(), trace);
        }
        
        for trace in active_traces {
            all_traces.insert(trace.id.clone(), trace);
        }
        
        let mut result: Vec<TraceRecord> = all_traces.into_values().collect();
        result.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        
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
            let all_traces = self.get_traces(None).await;
            
            if let Some(since) = since_id {
                if let Some(since_index) = all_traces.iter().position(|t| t.id == since) {
                    let result = all_traces[..since_index].to_vec();
                    if !result.is_empty() {
                        return result;
                    }
                } else {
                    if !all_traces.is_empty() {
                        return all_traces;
                    }
                }
            } else {
                if !all_traces.is_empty() {
                    return all_traces;
                }
            }
            
            if start_time.elapsed() >= timeout {
                return vec![];
            }
            
            sleep(check_interval).await;
        }
    }

    pub async fn clear(&self) {
        let mut active = self.active_traces.write().await;
        active.clear();
    }
}

