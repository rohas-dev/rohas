use crate::ws;
use axum::{
    extract::{ws::WebSocketUpgrade, ConnectInfo, MatchedPath, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post, put},
    Json, Router,
};
use std::net::SocketAddr;
use chrono::Utc;
use rohas_codegen::templates;
use rohas_parser::{HttpMethod, Schema};
use rohas_runtime::Executor;
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};
use tracing::{debug, info_span};

use crate::{config, EngineConfig};

#[derive(Clone)]
pub struct ApiState {
    pub executor: Arc<Executor>,
    pub schema: Arc<Schema>,
    pub config: Arc<EngineConfig>,
    pub event_bus: Arc<crate::event::EventBus>,
    pub trace_store: Arc<crate::telemetry::TraceStore>,
    pub tracing_log_store: Arc<crate::tracing_log::TracingLogStore>,
    pub workbench_auth: Arc<tokio::sync::RwLock<crate::workbench_auth::WorkbenchAuthConfig>>,
}

pub fn build_router(
    executor: Arc<Executor>,
    schema: Arc<Schema>,
    config: Arc<EngineConfig>,
    event_bus: Arc<crate::event::EventBus>,
    trace_store: Arc<crate::telemetry::TraceStore>,
    tracing_log_store: Arc<crate::tracing_log::TracingLogStore>,
) -> Router {
    let mut router = Router::new();
    let workbench_auth_config =
        crate::workbench_auth::WorkbenchAuthConfig::from_engine_config(&config);
    let workbench_auth = Arc::new(tokio::sync::RwLock::new(workbench_auth_config));
    let state = ApiState {
        executor,
        schema: schema.clone(),
        config,
        event_bus,
        trace_store,
        tracing_log_store,
        workbench_auth: workbench_auth.clone(),
    };
    

    for api in &schema.apis {
        let route_path = normalize_path(&api.path);

        debug!(
            "Adding route for API: {} {} -> handler: {}",
            api.method,
            route_path,
            templates::to_snake_case(api.name.as_str())
        );

        let handler_router = match api.method {
            HttpMethod::GET => Router::new().route(&route_path, get(api_handler)),
            HttpMethod::POST => Router::new().route(&route_path, post(api_handler)),
            HttpMethod::PUT => Router::new().route(&route_path, put(api_handler)),
            HttpMethod::PATCH => Router::new().route(&route_path, patch(api_handler)),
            HttpMethod::DELETE => Router::new().route(&route_path, delete(api_handler)),
        };

        router = router.merge(handler_router);
    }

    for ws in &schema.websockets {
        let route_path = normalize_path(&ws.path);
        debug!(
            "Adding websocket route: {} -> handler: {}",
            route_path,
            templates::to_snake_case(ws.name.as_str())
        );

        let ws_name = ws.name.clone();
        let handler_router =
            Router::new().route(
                &route_path,
                get(move |ws: WebSocketUpgrade, State(state): State<ApiState>| {
                    let ws_name = ws_name.clone();
                    async move {
                        ws.on_upgrade(move |socket| ws::websocket_handler(socket, state, ws_name))
                    }
                }),
            );

        router = router.merge(handler_router);
    }

    let workbench_router = crate::workbench::workbench_routes();
    let auth_config_for_middleware = workbench_auth.clone();
    let workbench_router = workbench_router.layer(axum::middleware::from_fn(move |request: Request, next: Next| {
        let auth_config = auth_config_for_middleware.clone();
        async move {
            crate::workbench_auth::workbench_auth_middleware(request, next, auth_config).await
        }
    }));
    router = router.merge(workbench_router);

    router.with_state(state)
}

/// Converts "/users/{id}" to "/users/:id" (Axum uses :param syntax)
fn normalize_path(path: &str) -> String {
    let mut result = String::new();
    let mut in_param = false;

    for ch in path.chars() {
        match ch {
            '{' => {
                in_param = true;
                result.push(':');
            }
            '}' => {
                in_param = false;
            }
            _ => {
                result.push(ch);
            }
        }
    }

    result
}

async fn api_handler(
    State(state): State<ApiState>,
    matched_path: Option<MatchedPath>,
    method: axum::http::Method,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
) -> Result<Response, ApiError> {
    let path_pattern = matched_path
        .as_ref()
        .map(|p| p.as_str())
        .ok_or_else(|| ApiError::Internal("No matched path".into()))?;

    let span = info_span!(
        "api_request",
        method = %method,
        path = %path_pattern,
    );
    let _enter = span.enter();

    debug!("Request received: {} {}", method, path_pattern);

    let mut metadata = HashMap::new();
    metadata.insert("method".to_string(), method.to_string());
    metadata.insert("path".to_string(), path_pattern.to_string());
    metadata.insert("datetime_utc".to_string(), Utc::now().to_rfc3339());

    // Extract IP address - check headers first, then fall back to remote address from extensions
    let ip_address = if let Some(ip) = request.headers().get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
    {
        Some(ip)
    } else if let Some(ip) = request.headers().get("x-real-ip")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
    {
        Some(ip)
    } else if let Some(ip) = request.headers().get("cf-connecting-ip")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
    {
        Some(ip)
    } else {
        // Fallback to remote address from ConnectInfo
        Some(addr.ip().to_string())
    };
    
    if let Some(ip) = ip_address {
        metadata.insert("ip_address".to_string(), ip);
    }
    
    if let Some(user_agent) = request.headers().get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
    {
        metadata.insert("user_agent".to_string(), user_agent);
    }

    if let Some(country) = request.headers().get("cf-ipcountry")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
    {
        metadata.insert("country".to_string(), country);
    }

    if let Some(city) = request.headers().get("cf-ipcity")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
    {
        metadata.insert("city".to_string(), city);
    }
    
    if let Some(region) = request.headers().get("cf-region")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
    {
        metadata.insert("region".to_string(), region);
    }

    let mut location_parts = Vec::new();
    if let Some(city) = metadata.get("city") {
        location_parts.push(city.clone());
    }
    if let Some(region) = metadata.get("region") {
        location_parts.push(region.clone());
    }
    if !location_parts.is_empty() {
        metadata.insert("location".to_string(), location_parts.join(", "));
    }
    
    let api_result = state
        .schema
        .apis
        .iter()
        .find(|api| {
            let normalized_path = normalize_path(&api.path);
            normalized_path == path_pattern && method_matches(&api.method, &method)
        });

    let api_name = api_result
        .map(|api| api.name.clone())
        .unwrap_or_else(|| format!("{} {}", method, path_pattern));
    
    let trace_id = state
        .trace_store
        .start_trace(api_name.clone(), crate::trace::TraceEntryType::Api, metadata)
        .await;

    tracing::Span::current().record("trace_id", &trace_id.as_str());

    let api = match api_result {
        Some(api) => api,
        None => {
            let error_msg = format!("No handler found for {} {}", method, path_pattern);
            state
                .trace_store
                .complete_trace(&trace_id, crate::trace::TraceStatus::Failed, Some(error_msg.clone()))
                .await;
            return Err(ApiError::NotFound(error_msg));
        }
    };

    let api_triggers = api.triggers.clone();
    let handler_name = match state.config.language {
        config::Language::TypeScript => api.name.clone(),
        config::Language::Python => templates::to_snake_case(api.name.clone().as_str()),
    };

    let api_path = api.path.clone();
    debug!("Matched handler: {}", handler_name);

    tracing::Span::current().record("api_name", &api.name.as_str());
    tracing::Span::current().record("handler_name", &handler_name.as_str());

    let normalized_api_path = normalize_path(&api_path);
    let path_params = extract_path_params(&normalized_api_path, request.uri().path());

    let query_params = request
        .uri()
        .query()
        .map(|q| parse_query_string(q))
        .unwrap_or_default();

    let body_bytes = match axum::body::to_bytes(request.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(e) => {
            let error_msg = format!("Failed to read body: {}", e);
            state
                .trace_store
                .complete_trace(&trace_id, crate::trace::TraceStatus::Failed, Some(error_msg.clone()))
                .await;
            return Err(ApiError::BadRequest(error_msg));
        }
    };

    let body_value = if body_bytes.is_empty() {
        Value::Object(serde_json::Map::new())
    } else {
        serde_json::from_slice(&body_bytes)
            .unwrap_or_else(|_| Value::Object(serde_json::Map::new()))
    };

    let mut payload = if let Value::Object(map) = body_value {
        Value::Object(map)
    } else {
        Value::Object(serde_json::Map::new())
    };

    if let Some(obj) = payload.as_object_mut() {
        for (key, value) in path_params {
            obj.insert(key, Value::String(value));
        }
    }

    let result = execute_handler(
        state.clone(),
        handler_name.clone(),
        payload,
        query_params,
        api_triggers,
        api_name,
        trace_id.clone(),
    )
    .await;

    match &result {
        Ok(_) => {
            state
                .trace_store
                .complete_trace(&trace_id, crate::trace::TraceStatus::Success, None)
                .await;
        }
        Err(e) => {
            let error_msg = match e {
                ApiError::BadRequest(msg) => Some(msg.clone()),
                ApiError::NotFound(msg) => Some(msg.clone()),
                ApiError::Internal(msg) => Some(msg.clone()),
            };
            state
                .trace_store
                .complete_trace(&trace_id, crate::trace::TraceStatus::Failed, error_msg)
                .await;
        }
    }

    result
}

fn method_matches(api_method: &HttpMethod, request_method: &axum::http::Method) -> bool {
    match api_method {
        HttpMethod::GET => request_method == axum::http::Method::GET,
        HttpMethod::POST => request_method == axum::http::Method::POST,
        HttpMethod::PUT => request_method == axum::http::Method::PUT,
        HttpMethod::PATCH => request_method == axum::http::Method::PATCH,
        HttpMethod::DELETE => request_method == axum::http::Method::DELETE,
    }
}

/// Example: pattern="/users/:id", path="/users/123" -> {"id": "123"}
fn extract_path_params(pattern: &str, path: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();
    let pattern_segments: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
    let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    if pattern_segments.len() != path_segments.len() {
        return params;
    }

    for (pattern_seg, path_seg) in pattern_segments.iter().zip(path_segments.iter()) {
        if let Some(param_name) = pattern_seg.strip_prefix(':') {
            params.insert(param_name.to_string(), path_seg.to_string());
        }
    }

    params
}

/// Example: "key1=value1&key2=value2" -> {"key1": "value1", "key2": "value2"}
fn parse_query_string(query: &str) -> HashMap<String, String> {
    query
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next()?.to_string();
            let value = parts.next().unwrap_or("").to_string();
            Some((key, value))
        })
        .collect()
}

async fn execute_handler(
    state: ApiState,
    handler_name: String,
    payload: Value,
    query_params: HashMap<String, String>,
    api_triggers: Vec<String>,
    api_name: String,
    trace_id: String,
) -> Result<Response, ApiError> {
    let handler_span = info_span!(
        "handler_execution",
        handler = %handler_name,
        trace_id = %trace_id,
    );
    let _enter = handler_span.enter();

    let start = std::time::Instant::now();
    let execution_result = state
        .executor
        .execute_with_params(&handler_name, payload, query_params)
        .await;

    let duration_ms = start.elapsed().as_millis() as u64;

    let exec_result = match execution_result {
        Ok(exec_result) => exec_result,
        Err(e) => {
            let error_msg = e.to_string();
            state
                .trace_store
                .add_step(
                    &trace_id,
                    handler_name.clone(),
                    duration_ms,
                    false,
                    Some(error_msg.clone()),
                )
                .await;
            return Err(ApiError::Internal(error_msg));
        }
    };

    let execution_time = exec_result.execution_time_ms.max(duration_ms);
    
    handler_span.record("duration_ms", execution_time);
    handler_span.record("success", exec_result.success);
    if let Some(ref error) = exec_result.error {
        handler_span.record("error", error.as_str());
    }

    let result = exec_result;

    let mut triggered_events = Vec::new();
    if result.success {
        for triggered_event in &result.triggers {
            let trigger_start = std::time::Instant::now();
            let emit_result = state
                .event_bus
                .emit(&triggered_event.event_name, triggered_event.payload.clone())
                .await;
            let trigger_duration = trigger_start.elapsed().as_millis() as u64;
            let trigger_timestamp = chrono::Utc::now().to_rfc3339();
            
            if let Err(e) = emit_result {
                tracing::error!(
                    "Failed to emit event {} from API {}: {}",
                    triggered_event.event_name,
                    api_name,
                    e
                );
            }
            
            triggered_events.push(crate::trace::TriggeredEventInfo {
                event_name: triggered_event.event_name.clone(),
                timestamp: trigger_timestamp,
                duration_ms: trigger_duration,
            });
        }
        let response_data = result.data.clone().unwrap_or(Value::Null);
        for trigger in &api_triggers {
            let trigger_start = std::time::Instant::now();
            let payload = result
                .auto_trigger_payloads
                .get(trigger)
                .cloned()
                .unwrap_or_else(|| response_data.clone());
            
            let emit_result = state.event_bus.emit(trigger, payload).await;
            let trigger_duration = trigger_start.elapsed().as_millis() as u64;
            let trigger_timestamp = chrono::Utc::now().to_rfc3339();
            
            if let Err(e) = emit_result {
                tracing::error!(
                    "Failed to emit auto-triggered event {} from API {}: {}",
                    trigger,
                    api_name,
                    e
                );
            }
            
            triggered_events.push(crate::trace::TriggeredEventInfo {
                event_name: trigger.clone(),
                timestamp: trigger_timestamp,
                duration_ms: trigger_duration,
            });
        }
    }

    state
        .trace_store
        .add_step_with_triggers(
            &trace_id,
            handler_name.clone(),
            execution_time,
            result.success,
            result.error.clone(),
            triggered_events.clone(),
        )
        .await;

    if result.success {
        let response_data = result.data.clone().unwrap_or(Value::Null);

        for triggered_event in &result.triggers {
            if let Err(e) = state
                .event_bus
                .emit(&triggered_event.event_name, triggered_event.payload.clone())
                .await
            {
                tracing::error!(
                    "Failed to emit event {} from API {}: {}",
                    triggered_event.event_name,
                    api_name,
                    e
                );
            }
        }

        for trigger in &api_triggers {
            let payload = result
                .auto_trigger_payloads
                .get(trigger)
                .cloned()
                .unwrap_or_else(|| response_data.clone());

            if let Err(e) = state.event_bus.emit(trigger, payload).await {
                tracing::error!(
                    "Failed to emit auto-triggered event {} from API {}: {}",
                    trigger,
                    api_name,
                    e
                );
            }
        }

        Ok((StatusCode::OK, Json(response_data)).into_response())
    } else {
        let error_msg = result.error.unwrap_or_else(|| "Unknown error".to_string());
        Err(ApiError::Internal(error_msg))
    }
}

#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    NotFound(String),
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = serde_json::json!({
            "error": message,
        });

        (status, Json(body)).into_response()
    }
}
