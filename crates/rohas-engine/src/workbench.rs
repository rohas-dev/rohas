use crate::api::ApiState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::Path as StdPath;
use tracing::error;

#[derive(Serialize, Deserialize)]
pub struct ProjectConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<ProjectInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<ServerInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
pub struct ProjectInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ServerInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
}

#[derive(Serialize, Deserialize)]
pub struct ProjectFile {
    pub name: String,
    pub relative_path: String,
    pub size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct SchemaBucket {
    pub name: String,
    pub files: Vec<ProjectFile>,
}

#[derive(Serialize, Deserialize)]
pub struct HandlerBucket {
    pub name: String,
    pub files: Vec<ProjectFile>,
}

#[derive(Serialize, Deserialize)]
pub struct ProjectSnapshot {
    pub root: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<ProjectConfig>,
    pub schema: SchemaInfo,
    pub handlers: HandlerInfo,
}

#[derive(Serialize, Deserialize)]
pub struct SchemaInfo {
    pub total: usize,
    pub buckets: Vec<SchemaBucket>,
}

#[derive(Serialize, Deserialize)]
pub struct HandlerInfo {
    pub total: usize,
    pub buckets: Vec<HandlerBucket>,
}

#[derive(Serialize, Deserialize)]
pub struct EntityRow {
    pub name: String,
    pub bucket: String,
    pub path: String,
    pub size: String,
}

#[derive(Serialize, Deserialize)]
pub struct ActivityItem {
    pub id: String,
    pub title: String,
    pub description: String,
    pub timestamp: String,
}

#[derive(Serialize, Deserialize)]
pub struct WorkbenchData {
    pub root: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<ProjectConfig>,
    pub schema_total: usize,
    pub schema_bucket_count: usize,
    pub handler_total: usize,
    pub schema_rows: Vec<EntityRow>,
    pub handler_rows: Vec<EntityRow>,
    pub activity: Vec<ActivityItem>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SchemaGraphNode {
    pub id: String,
    pub label: String,
    pub bucket: String,
    pub path: String,
    #[serde(rename = "type")]
    pub node_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct SchemaGraphEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub relation: String,
}

#[derive(Serialize, Deserialize)]
pub struct SchemaGraph {
    pub root: String,
    pub nodes: Vec<SchemaGraphNode>,
    pub edges: Vec<SchemaGraphEdge>,
}

#[derive(Serialize, Deserialize)]
pub struct TraceStep {
    pub name: String,
    pub path: String,
    pub bucket: String,
    pub handler_name: String,
    pub duration_ms: u64,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub triggered_events: Vec<crate::trace::TriggeredEventInfo>,
}

#[derive(Serialize, Deserialize)]
pub struct TraceRecord {
    pub id: String,
    pub entry_point: String,
    pub entry_type: String,
    pub bucket: String,
    pub status: String,
    pub duration_ms: u64,
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    pub steps: Vec<TraceStep>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

pub fn workbench_routes() -> Router<ApiState> {
    Router::new()
        .route("/api/workbench/snapshot", get(get_snapshot))
        .route("/api/workbench/data", get(get_workbench_data))
        .route("/api/workbench/schema-graph", get(get_schema_graph))
        .route("/api/workbench/traces", get(get_traces))
        .route("/api/workbench/traces/poll", get(poll_traces))
        .route("/api/workbench/logs", get(get_tracing_logs))
        .route("/api/workbench/logs/poll", get(poll_tracing_logs))
        .route("/api/workbench/endpoints", get(get_endpoints))
        .route("/api/workbench/types/{type_name}", get(get_type_schema))
        .route("/api/workbench/events/{name}/trigger", post(trigger_event))
}

async fn get_snapshot(State(state): State<ApiState>) -> Result<Response, WorkbenchError> {
    let snapshot = load_project_snapshot(&state.config.project_root)?;
    Ok(Json(snapshot).into_response())
}

async fn get_workbench_data(State(state): State<ApiState>) -> Result<Response, WorkbenchError> {
    let snapshot = load_project_snapshot(&state.config.project_root)?;
    let schema_rows = flatten_schema_buckets(&snapshot.schema.buckets, 50);
    let handler_rows = flatten_handler_buckets(&snapshot.handlers.buckets, 50);
    let activity = build_activity_feed(&schema_rows, &handler_rows);

    let data = WorkbenchData {
        root: snapshot.root,
        config: snapshot.config,
        schema_total: snapshot.schema.total,
        schema_bucket_count: snapshot.schema.buckets.len(),
        handler_total: snapshot.handlers.total,
        schema_rows,
        handler_rows,
        activity,
    };

    Ok(Json(data).into_response())
}

async fn get_schema_graph(State(state): State<ApiState>) -> Result<Response, WorkbenchError> {
    let snapshot = load_project_snapshot(&state.config.project_root)?;
    let graph = build_schema_graph(&snapshot)?;
    Ok(Json(graph).into_response())
}

#[derive(Deserialize)]
struct TracesQuery {
    limit: Option<usize>,
}

async fn get_traces(
    State(state): State<ApiState>,
    Query(params): Query<TracesQuery>,
) -> Result<Response, WorkbenchError> {
    let traces = state.trace_store.get_traces(params.limit).await;
    
    let api_traces: Vec<TraceRecord> = traces
        .into_iter()
        .map(|trace| TraceRecord {
            id: trace.id,
            entry_point: trace.entry_point,
            entry_type: match trace.entry_type {
                crate::trace::TraceEntryType::Api => "api".to_string(),
                crate::trace::TraceEntryType::Event => "event".to_string(),
                crate::trace::TraceEntryType::Cron => "cron".to_string(),
                crate::trace::TraceEntryType::WebSocket => "websocket".to_string(),
            },
            bucket: trace
                .metadata
                .get("path")
                .cloned()
                .unwrap_or_else(|| "unknown".to_string()),
            status: match trace.status {
                crate::trace::TraceStatus::Success => "success".to_string(),
                crate::trace::TraceStatus::Failed => "failed".to_string(),
                crate::trace::TraceStatus::Running => "running".to_string(),
            },
            duration_ms: trace.duration_ms,
            started_at: trace.started_at,
            completed_at: trace.completed_at,
            error: trace.error,
            metadata: trace.metadata,
            steps: trace
                .steps
                .into_iter()
                .map(|step| TraceStep {
                    name: step.name,
                    path: step.handler_name.clone(),
                    bucket: step.handler_name.clone(),
                    handler_name: step.handler_name,
                    duration_ms: step.duration_ms,
                    success: step.success,
                    error: step.error,
                    timestamp: step.timestamp,
                    triggered_events: step.triggered_events,
                })
                .collect(),
        })
        .collect();

    Ok(Json(api_traces).into_response())
}

#[derive(Deserialize)]
struct PollTracesQuery {
    since: Option<String>,
    timeout: Option<u64>,
}

async fn poll_traces(
    State(state): State<ApiState>,
    Query(params): Query<PollTracesQuery>,
) -> Result<Response, WorkbenchError> {
    let timeout = params.timeout.unwrap_or(30); // Default 30 seconds
    let traces = state
        .trace_store
        .get_traces_since(params.since.as_deref(), timeout)
        .await;
    
    let api_traces: Vec<TraceRecord> = traces
        .into_iter()
        .map(|trace| TraceRecord {
            id: trace.id,
            entry_point: trace.entry_point,
            entry_type: match trace.entry_type {
                crate::trace::TraceEntryType::Api => "api".to_string(),
                crate::trace::TraceEntryType::Event => "event".to_string(),
                crate::trace::TraceEntryType::Cron => "cron".to_string(),
                crate::trace::TraceEntryType::WebSocket => "websocket".to_string(),
            },
            bucket: trace
                .metadata
                .get("path")
                .cloned()
                .unwrap_or_else(|| "unknown".to_string()),
            status: match trace.status {
                crate::trace::TraceStatus::Success => "success".to_string(),
                crate::trace::TraceStatus::Failed => "failed".to_string(),
                crate::trace::TraceStatus::Running => "running".to_string(),
            },
            duration_ms: trace.duration_ms,
            started_at: trace.started_at,
            completed_at: trace.completed_at,
            error: trace.error,
            metadata: trace.metadata,
            steps: trace
                .steps
                .into_iter()
                .map(|step| TraceStep {
                    name: step.name,
                    path: step.handler_name.clone(),
                    bucket: step.handler_name.clone(),
                    handler_name: step.handler_name,
                    duration_ms: step.duration_ms,
                    success: step.success,
                    error: step.error,
                    timestamp: step.timestamp,
                    triggered_events: step.triggered_events,
                })
                .collect(),
        })
        .collect();

    Ok(Json(api_traces).into_response())
}

#[derive(Deserialize)]
struct LogsQuery {
    limit: Option<usize>,
    level: Option<String>,
}

async fn get_tracing_logs(
    State(state): State<ApiState>,
    Query(params): Query<LogsQuery>,
) -> Result<Response, WorkbenchError> {
    let logs = state
        .tracing_log_store
        .get_logs(params.limit, params.level.as_deref())
        .await;
    
    Ok(Json(logs).into_response())
}

#[derive(Deserialize)]
struct PollLogsQuery {
    since: Option<String>, // timestamp
    level: Option<String>,
    timeout: Option<u64>,
}

async fn poll_tracing_logs(
    State(state): State<ApiState>,
    Query(params): Query<PollLogsQuery>,
) -> Result<Response, WorkbenchError> {
    use tokio::time::{sleep, Duration};
    
    let timeout = params.timeout.unwrap_or(30);
    let start_time = std::time::Instant::now();
    let timeout_duration = Duration::from_secs(timeout);
    let check_interval = Duration::from_millis(500);
    
    loop {
        let logs = state
            .tracing_log_store
            .get_logs(None, params.level.as_deref())
            .await;
        
        let filtered_logs: Vec<_> = if let Some(since) = &params.since {
            logs.into_iter()
                .filter(|log| log.timestamp > *since)
                .collect()
        } else {
            logs
        };
        
        if !filtered_logs.is_empty() {
            return Ok(Json(filtered_logs).into_response());
        }
        
        if start_time.elapsed() >= timeout_duration {
            return Ok(Json(Vec::<crate::tracing_log::TracingLogEntry>::new()).into_response());
        }
        
        sleep(check_interval).await;
    }
}

fn load_project_snapshot(project_root: &StdPath) -> Result<ProjectSnapshot, WorkbenchError> {
    let root = project_root.to_string_lossy().to_string();
    let config = read_project_config(project_root).ok();
    let schema = read_schema_buckets(project_root)?;
    let handlers = read_handler_buckets(project_root)?;

    Ok(ProjectSnapshot {
        root,
        config,
        schema,
        handlers,
    })
}

fn read_project_config(project_root: &StdPath) -> Result<ProjectConfig, WorkbenchError> {
    let config_path = project_root.join("config").join("rohas.toml");
    if !config_path.exists() {
        return Err(WorkbenchError::NotFound("Config file not found".to_string()));
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|e| WorkbenchError::Internal(format!("Failed to read config: {}", e)))?;
    let toml_value: toml::Value = toml::from_str(&content)
        .map_err(|e| WorkbenchError::Internal(format!("Failed to parse config: {}", e)))?;

    let project = toml_value
        .get("project")
        .and_then(|p| {
            Some(ProjectInfo {
                name: p.get("name")?.as_str().map(|s| s.to_string()),
                version: p.get("version")?.as_str().map(|s| s.to_string()),
                language: p.get("language")?.as_str().map(|s| s.to_string()),
            })
        });

    let server = toml_value
        .get("server")
        .and_then(|s| {
            Some(ServerInfo {
                host: s.get("host")?.as_str().map(|s| s.to_string()),
                port: s.get("port")?.as_integer().and_then(|p| u16::try_from(p).ok()),
            })
        });

    let adapter = toml_value.get("adapter").map(|a| {
        serde_json::to_value(a).unwrap_or(serde_json::Value::Null)
    });

    Ok(ProjectConfig {
        project,
        server,
        adapter,
    })
}

fn read_schema_buckets(project_root: &StdPath) -> Result<SchemaInfo, WorkbenchError> {
    let schema_dir = project_root.join("schema");
    let buckets = collect_buckets(project_root, &schema_dir, true, &[".ro"])?;

    Ok(SchemaInfo {
        total: buckets.iter().map(|b| b.files.len()).sum(),
        buckets,
    })
}

fn read_handler_buckets(project_root: &StdPath) -> Result<HandlerInfo, WorkbenchError> {
    let handlers_dir = project_root.join("src").join("handlers");
    let buckets = collect_buckets(project_root, &handlers_dir, true, &[".py", ".ts", ".js", ".tsx"])?;

    Ok(HandlerInfo {
        total: buckets.iter().map(|b| b.files.len()).sum(),
        buckets: buckets
            .into_iter()
            .map(|b| HandlerBucket {
                name: b.name,
                files: b.files,
            })
            .collect(),
    })
}

fn collect_buckets(
    project_root: &StdPath,
    base_dir: &StdPath,
    capture_content: bool,
    extensions: &[&str],
) -> Result<Vec<SchemaBucket>, WorkbenchError> {
    if !base_dir.exists() {
        return Ok(vec![]);
    }

    let entries = fs::read_dir(base_dir)
        .map_err(|e| WorkbenchError::Internal(format!("Failed to read directory: {}", e)))?;

    let mut buckets = Vec::new();

    for entry in entries {
        let entry = entry
            .map_err(|e| WorkbenchError::Internal(format!("Failed to read entry: {}", e)))?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let files = collect_files(project_root, &path, capture_content, extensions)?;
        if files.is_empty() {
            continue;
        }

        buckets.push(SchemaBucket {
            name: entry
                .file_name()
                .to_string_lossy()
                .to_string(),
            files,
        });
    }

    buckets.sort_by(|a, b| b.files.len().cmp(&a.files.len()));

    Ok(buckets)
}

fn collect_files(
    project_root: &StdPath,
    dir: &StdPath,
    capture_content: bool,
    extensions: &[&str],
) -> Result<Vec<ProjectFile>, WorkbenchError> {
    if !dir.exists() {
        return Ok(vec![]);
    }

    let entries = fs::read_dir(dir)
        .map_err(|e| WorkbenchError::Internal(format!("Failed to read directory: {}", e)))?;

    let mut files = Vec::new();

    for entry in entries {
        let entry = entry
            .map_err(|e| WorkbenchError::Internal(format!("Failed to read entry: {}", e)))?;
        let path = entry.path();

        if path.is_dir() {
            files.extend(collect_files(project_root, &path, capture_content, extensions)?);
            continue;
        }

        let file_name = entry.file_name().to_string_lossy().to_string();
        if !extensions.iter().any(|ext| file_name.to_lowercase().ends_with(ext)) {
            continue;
        }

        let metadata = fs::metadata(&path)
            .map_err(|e| WorkbenchError::Internal(format!("Failed to read metadata: {}", e)))?;

        let content = if capture_content {
            fs::read_to_string(&path)
                .map_err(|e| {
                    error!("Failed to read file {}: {}", path.display(), e);
                })
                .ok()
        } else {
            None
        };

        let relative_path = path
            .strip_prefix(project_root)
            .map_err(|_| {
                WorkbenchError::Internal(format!(
                    "Failed to compute relative path for {}",
                    path.display()
                ))
            })?
            .to_string_lossy()
            .to_string();

        files.push(ProjectFile {
            name: file_name,
            relative_path,
            size: metadata.len(),
            content,
        });
    }

    Ok(files)
}

fn flatten_schema_buckets(buckets: &[SchemaBucket], limit: usize) -> Vec<EntityRow> {
    let mut rows = Vec::new();

    for bucket in buckets {
        for file in &bucket.files {
            rows.push(EntityRow {
                name: file.name.clone(),
                bucket: bucket.name.clone(),
                path: file.relative_path.clone(),
                size: format_bytes(file.size),
            });

            if rows.len() >= limit {
                return rows;
            }
        }
    }

    rows
}

fn flatten_handler_buckets(buckets: &[HandlerBucket], limit: usize) -> Vec<EntityRow> {
    let mut rows = Vec::new();

    for bucket in buckets {
        for file in &bucket.files {
            rows.push(EntityRow {
                name: file.name.clone(),
                bucket: bucket.name.clone(),
                path: file.relative_path.clone(),
                size: format_bytes(file.size),
            });

            if rows.len() >= limit {
                return rows;
            }
        }
    }

    rows
}

fn build_activity_feed(schema_rows: &[EntityRow], handler_rows: &[EntityRow]) -> Vec<ActivityItem> {
    let mut events = Vec::new();

    for (index, row) in schema_rows.iter().enumerate() {
        events.push(ActivityItem {
            id: format!("schema-{}", row.path),
            title: format!("Schema {}", row.name),
            description: format!("Category {} ({})", row.bucket, row.path),
            timestamp: format!("{} min ago", index + 1),
        });
    }

    for (index, row) in handler_rows.iter().enumerate() {
        events.push(ActivityItem {
            id: format!("handler-{}", row.path),
            title: format!("Handler {}", row.name),
            description: format!("Category {} ({})", row.bucket, row.path),
            timestamp: format!("{} min ago", index + schema_rows.len() + 1),
        });
    }

    if events.is_empty() {
        return vec![ActivityItem {
            id: "bootstrap".to_string(),
            title: "Workspace bootstrap".to_string(),
            description: "Add schema files or handlers to this project to populate the feed."
                .to_string(),
            timestamp: "just now".to_string(),
        }];
    }

    events.truncate(12);
    events
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        return format!("{} B", bytes);
    }

    let units = ["KB", "MB", "GB"];
    let mut value = bytes as f64 / 1024.0;
    let mut unit_index = 0;

    while value >= 1024.0 && unit_index < units.len() - 1 {
        value /= 1024.0;
        unit_index += 1;
    }

    format!("{:.1} {}", value, units[unit_index])
}


fn build_schema_graph(snapshot: &ProjectSnapshot) -> Result<SchemaGraph, WorkbenchError> {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut name_map = HashMap::new();

    for bucket in &snapshot.schema.buckets {
        for file in &bucket.files {
            let base_name = file.name.replace(".ro", "").replace(".RO", "");
            let node = SchemaGraphNode {
                id: format!("{}-{}", bucket.name, base_name),
                label: base_name.clone(),
                bucket: bucket.name.clone(),
                path: file.relative_path.clone(),
                node_type: "schema".to_string(),
                source: file.content.clone(),
            };
            nodes.push(node.clone());
            name_map.insert(base_name.to_lowercase(), node);
        }
    }

    for bucket in &snapshot.handlers.buckets {
        for file in &bucket.files {
            let base_name = file
                .name
                .replace(".ts", "")
                .replace(".tsx", "")
                .replace(".js", "")
                .replace(".py", "");
            let node = SchemaGraphNode {
                id: format!("handler-{}-{}", bucket.name, base_name),
                label: base_name.clone(),
                bucket: format!("handler/{}", bucket.name),
                path: file.relative_path.clone(),
                node_type: "handler".to_string(),
                source: file.content.clone(),
            };
            nodes.push(node.clone());
            name_map.insert(base_name.to_lowercase(), node);
        }
    }

    // Build edges from schema references
    for node in &nodes {
        if node.node_type == "schema" {
            if let Some(content) = &node.source {
                let references = extract_references(content, &name_map);
                for (target, relation) in references {
                    if let Some(target_node) = name_map.get(&target) {
                        if target_node.id != node.id {
                            edges.push(SchemaGraphEdge {
                                id: format!("{}-{}-{}", node.id, target_node.id, edges.len()),
                                source: node.id.clone(),
                                target: target_node.id.clone(),
                                relation,
                            });
                        }
                    }
                }
            }
        } else if node.node_type == "handler" {
            for schema_bucket in &snapshot.schema.buckets {
                for schema_file in &schema_bucket.files {
                    if let Some(content) = &schema_file.content {
                        let handler_regex = regex::Regex::new(r"handler\s*:\s*\[([^\]]*)\]")
                            .map_err(|e| WorkbenchError::Internal(format!("Regex error: {}", e)))?;
                        
                        for cap in handler_regex.captures_iter(content) {
                            if let Some(list_str) = cap.get(1) {
                                let list: Vec<&str> = list_str
                                    .as_str()
                                    .split(',')
                                    .map(|s| s.trim().trim_matches(|c| c == '"' || c == '\''))
                                    .filter(|s| !s.is_empty())
                                    .collect();

                                if list.iter().any(|name| {
                                    name.to_lowercase() == node.label.to_lowercase()
                                }) {
                                    if let Some(schema_node) = nodes.iter().find(|n| {
                                        n.node_type == "schema"
                                            && n.path == schema_file.relative_path
                                    }) {
                                        edges.push(SchemaGraphEdge {
                                            id: format!(
                                                "{}-{}-{}",
                                                schema_node.id,
                                                node.id,
                                                edges.len()
                                            ),
                                            source: schema_node.id.clone(),
                                            target: node.id.clone(),
                                            relation: "handler".to_string(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(SchemaGraph {
        root: snapshot.root.clone(),
        nodes,
        edges,
    })
}

fn extract_references(
    content: &str,
    name_map: &HashMap<String, SchemaGraphNode>,
) -> Vec<(String, String)> {
    let mut references = Vec::new();
    let mut seen = std::collections::HashSet::new();

    let trigger_regex = regex::Regex::new(r"triggers\s*:\s*\[([^\]]*)\]")
        .unwrap_or_else(|_| regex::Regex::new(r"triggers\s*:\s*\[\]").unwrap());
    
    for cap in trigger_regex.captures_iter(content) {
        if let Some(list_str) = cap.get(1) {
            let list: Vec<&str> = list_str
                .as_str()
                .split(',')
                .map(|s| s.trim().trim_matches(|c| c == '"' || c == '\''))
                .filter(|s| !s.is_empty())
                .collect();

            for name in list {
                let normalized = name.to_lowercase();
                let key = format!("triggers:{}", normalized);
                if !seen.contains(&key) && name_map.contains_key(&normalized) {
                    references.push((normalized, "triggers".to_string()));
                    seen.insert(key);
                }
            }
        }
    }
 
    let word_regex = regex::Regex::new(r"\b[a-zA-Z0-9_]+\b").unwrap();
    for cap in word_regex.find_iter(content) {
        let token = cap.as_str().to_lowercase();
        let key = format!("references:{}", token);
        if !seen.contains(&key) && name_map.contains_key(&token) {
            references.push((token, "references".to_string()));
            seen.insert(key);
        }
    }

    references
}

#[derive(Serialize, Deserialize)]
pub struct ApiEndpoint {
    pub name: String,
    pub method: String,
    pub path: String,
    pub body: Option<String>,
    pub response: String,
    pub triggers: Vec<String>,
    pub middlewares: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct WebSocketEndpoint {
    pub name: String,
    pub path: String,
    pub message: Option<String>,
    pub on_connect: Vec<String>,
    pub on_message: Vec<String>,
    pub on_disconnect: Vec<String>,
    pub triggers: Vec<String>,
    pub broadcast: bool,
    pub middlewares: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct CronJob {
    pub name: String,
    pub schedule: String,
    pub triggers: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct EventEndpoint {
    pub name: String,
    pub payload: String,
    pub handlers: Vec<String>,
    pub triggers: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct EndpointsData {
    pub apis: Vec<ApiEndpoint>,
    pub websockets: Vec<WebSocketEndpoint>,
    pub crons: Vec<CronJob>,
    pub events: Vec<EventEndpoint>,
}

async fn get_endpoints(State(state): State<ApiState>) -> Result<Response, WorkbenchError> {
    let apis: Vec<ApiEndpoint> = state
        .schema
        .apis
        .iter()
        .map(|api| ApiEndpoint {
            name: api.name.clone(),
            method: api.method.to_string(),
            path: api.path.clone(),
            body: api.body.clone(),
            response: api.response.clone(),
            triggers: api.triggers.clone(),
            middlewares: api.middlewares.clone(),
        })
        .collect();

    let websockets: Vec<WebSocketEndpoint> = state
        .schema
        .websockets
        .iter()
        .map(|ws| WebSocketEndpoint {
            name: ws.name.clone(),
            path: ws.path.clone(),
            message: ws.message.clone(),
            on_connect: ws.on_connect.clone(),
            on_message: ws.on_message.clone(),
            on_disconnect: ws.on_disconnect.clone(),
            triggers: ws.triggers.clone(),
            broadcast: ws.broadcast,
            middlewares: ws.middlewares.clone(),
        })
        .collect();

    let crons: Vec<CronJob> = state
        .schema
        .crons
        .iter()
        .map(|cron| CronJob {
            name: cron.name.clone(),
            schedule: cron.schedule.clone(),
            triggers: cron.triggers.clone(),
        })
        .collect();

    let events: Vec<EventEndpoint> = state
        .schema
        .events
        .iter()
        .map(|event| EventEndpoint {
            name: event.name.clone(),
            payload: event.payload.clone(),
            handlers: event.handlers.clone(),
            triggers: event.triggers.clone(),
        })
        .collect();

    let data = EndpointsData {
        apis,
        websockets,
        crons,
        events,
    };

    Ok(Json(data).into_response())
}

#[derive(Deserialize)]
struct TriggerEventRequest {
    payload: Option<serde_json::Value>,
}

async fn get_type_schema(
    Path(type_name): Path<String>,
    State(state): State<ApiState>,
) -> Result<Response, WorkbenchError> {
    use rohas_parser::FieldType;

    // Check if it's a primitive type
    let json_schema = match type_name.as_str() {
        "String" => json!({
            "type": "string"
        }),
        "Int" | "Float" => json!({
            "type": "number"
        }),
        "Boolean" | "Bool" => json!({
            "type": "boolean"
        }),
        "DateTime" | "Date" => json!({
            "type": "string",
            "format": "date-time"
        }),
        "Json" => json!({
            "type": "object"
        }),
        _ => {
            if let Some(input) = state.schema.inputs.iter().find(|i| i.name == type_name) {
                let mut properties = serde_json::Map::new();
                let mut required = Vec::new();

                for field in &input.fields {
                    let field_schema = match &field.field_type {
                        FieldType::String => json!({ "type": "string" }),
                        FieldType::Int | FieldType::Float => json!({ "type": "number" }),
                        FieldType::Boolean => json!({ "type": "boolean" }),
                        FieldType::DateTime => json!({
                            "type": "string",
                            "format": "date-time"
                        }),
                        FieldType::Json => json!({ "type": "object" }),
                        FieldType::Custom(_custom_type) => {
                            json!({ "type": "object" })
                        }
                        FieldType::Array(inner) => {
                            let inner_schema = match inner.as_ref() {
                                FieldType::String => json!({ "type": "string" }),
                                FieldType::Int | FieldType::Float => json!({ "type": "number" }),
                                FieldType::Boolean => json!({ "type": "boolean" }),
                                FieldType::DateTime => json!({
                                    "type": "string",
                                    "format": "date-time"
                                }),
                                FieldType::Json => json!({ "type": "object" }),
                                _ => json!({ "type": "object" }),
                            };
                            json!({
                                "type": "array",
                                "items": inner_schema
                            })
                        }
                    };

                    properties.insert(field.name.clone(), field_schema);
                    if !field.optional {
                        required.push(field.name.clone());
                    }
                }

                let mut schema = json!({
                    "type": "object",
                    "properties": properties
                });
                if !required.is_empty() {
                    schema.as_object_mut().unwrap().insert("required".to_string(), json!(required));
                }
                schema
            } else if let Some(model) = state.schema.models.iter().find(|m| m.name == type_name) {
                let mut properties = serde_json::Map::new();
                let mut required = Vec::new();

                for field in &model.fields {
                    let field_schema = match &field.field_type {
                        FieldType::String => json!({ "type": "string" }),
                        FieldType::Int | FieldType::Float => json!({ "type": "number" }),
                        FieldType::Boolean => json!({ "type": "boolean" }),
                        FieldType::DateTime => json!({
                            "type": "string",
                            "format": "date-time"
                        }),
                        FieldType::Json => json!({ "type": "object" }),
                        FieldType::Custom(custom_type) => {
                            json!({ "type": "object" })
                        }
                        FieldType::Array(inner) => {
                            let inner_schema = match inner.as_ref() {
                                FieldType::String => json!({ "type": "string" }),
                                FieldType::Int | FieldType::Float => json!({ "type": "number" }),
                                FieldType::Boolean => json!({ "type": "boolean" }),
                                FieldType::DateTime => json!({
                                    "type": "string",
                                    "format": "date-time"
                                }),
                                FieldType::Json => json!({ "type": "object" }),
                                _ => json!({ "type": "object" }),
                            };
                            json!({
                                "type": "array",
                                "items": inner_schema
                            })
                        }
                    };

                    properties.insert(field.name.clone(), field_schema);
                    if !field.optional {
                        required.push(field.name.clone());
                    }
                }

                let mut schema = json!({
                    "type": "object",
                    "properties": properties
                });
                if !required.is_empty() {
                    schema.as_object_mut().unwrap().insert("required".to_string(), json!(required));
                }
                schema
            } else {
                return Err(WorkbenchError::NotFound(format!("Type '{}' not found", type_name)));
            }
        }
    };

    Ok(Json(json_schema).into_response())
}

async fn trigger_event(
    Path(event_name): Path<String>,
    State(state): State<ApiState>,
    Json(request): Json<TriggerEventRequest>,
) -> Result<Response, WorkbenchError> {
    let payload = request.payload.unwrap_or_else(|| serde_json::json!({}));

    state
        .event_bus
        .emit(&event_name, payload.clone())
        .await
        .map_err(|e| WorkbenchError::Internal(format!("Failed to emit event: {}", e)))?;

    Ok(Json(json!({
        "success": true,
        "event": event_name,
        "payload": payload,
    }))
    .into_response())
}

#[derive(Debug)]
pub enum WorkbenchError {
    NotFound(String),
    Internal(String),
}

impl IntoResponse for WorkbenchError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            WorkbenchError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            WorkbenchError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = serde_json::json!({
            "error": message,
        });

        (status, Json(body)).into_response()
    }
}

