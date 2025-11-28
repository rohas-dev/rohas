use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;
use tracing_subscriber::reload::Handle;
use tracing_subscriber::Registry;

static TRACING_LAYER_HANDLE: std::sync::OnceLock<Arc<Handle<Option<TracingLogLayer>, Registry>>> = std::sync::OnceLock::new();

pub fn set_tracing_layer_handle(handle: Arc<Handle<Option<TracingLogLayer>, Registry>>) {
    let _ = TRACING_LAYER_HANDLE.set(handle);
}

pub fn register_tracing_log_layer(layer: TracingLogLayer) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(handle) = TRACING_LAYER_HANDLE.get() {
        handle.reload(Some(layer))?;
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingLogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
    pub fields: HashMap<String, String>,
    pub span_name: Option<String>,
    pub span_fields: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
}

pub struct TracingLogStore {
    logs: Arc<RwLock<Vec<TracingLogEntry>>>,
    max_logs: usize,
}

impl TracingLogStore {
    pub fn new(max_logs: usize) -> Self {
        Self {
            logs: Arc::new(RwLock::new(Vec::new())),
            max_logs,
        }
    }

    pub async fn add_log(&self, entry: TracingLogEntry) {
        let mut logs = self.logs.write().await;
        logs.push(entry);
        
        if logs.len() > self.max_logs {
            logs.remove(0);
        }
    }

    pub async fn get_logs(&self, limit: Option<usize>, level_filter: Option<&str>) -> Vec<TracingLogEntry> {
        let logs = self.logs.read().await;
        let mut result: Vec<TracingLogEntry> = logs.iter().rev().cloned().collect();
        
        if let Some(level) = level_filter {
            result.retain(|log| log.level == level);
        }
        
        if let Some(limit) = limit {
            result.truncate(limit);
        }
        
        result
    }

    pub async fn clear(&self) {
        let mut logs = self.logs.write().await;
        logs.clear();
    }
}

pub struct TracingLogLayer {
    store: Arc<TracingLogStore>,
}

impl TracingLogLayer {
    pub fn new(store: Arc<TracingLogStore>) -> Self {
        Self { store }
    }
}

impl<S> Layer<S> for TracingLogLayer
where
    S: Subscriber + for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>,
{
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let level = match *metadata.level() {
            Level::ERROR => "error",
            Level::WARN => "warn",
            Level::INFO => "info",
            Level::DEBUG => "debug",
            Level::TRACE => "trace",
        };

        let mut fields = HashMap::new();
        let mut visitor = FieldVisitor::new(&mut fields);
        event.record(&mut visitor);

        let span_name = ctx
            .lookup_current()
            .map(|span| span.metadata().name().to_string());
        
        let span_fields = HashMap::new();

        let message = if fields.is_empty() {
            metadata.name().to_string()
        } else {
            fields.get("message")
                .or_else(|| fields.get("msg"))
                .cloned()
                .unwrap_or_else(|| {
                    // Format as key=value pairs
                    fields.iter()
                        .map(|(k, v)| format!("{}={}", k, v))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
        };

        let entry = TracingLogEntry {
            timestamp: Utc::now().to_rfc3339(),
            level: level.to_string(),
            target: metadata.target().to_string(),
            message,
            fields,
            span_name,
            span_fields,
            file: metadata.file().map(|f| f.to_string()),
            line: metadata.line(),
        };

        let store = self.store.clone();
        tokio::spawn(async move {
            store.add_log(entry).await;
        });
    }
}

struct FieldVisitor<'a> {
    fields: &'a mut HashMap<String, String>,
}

impl<'a> FieldVisitor<'a> {
    fn new(fields: &'a mut HashMap<String, String>) -> Self {
        Self { fields }
    }
}

impl<'a> tracing::field::Visit for FieldVisitor<'a> {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.fields.insert(field.name().to_string(), value.to_string());
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.fields.insert(field.name().to_string(), format!("{:?}", value));
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.fields.insert(field.name().to_string(), value.to_string());
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.fields.insert(field.name().to_string(), value.to_string());
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.fields.insert(field.name().to_string(), value.to_string());
    }
}

