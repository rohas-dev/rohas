use axum::{
    extract::{MatchedPath, Request, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post, put},
    Json, Router,
};
use rohas_codegen::templates;
use rohas_parser::{HttpMethod, Schema};
use rohas_runtime::Executor;
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc};
use tracing::debug;

use crate::{config, EngineConfig};

#[derive(Clone)]
pub struct ApiState {
    pub executor: Arc<Executor>,
    pub schema: Arc<Schema>,
    pub config: Arc<EngineConfig>,
}

pub fn build_router(
    executor: Arc<Executor>,
    schema: Arc<Schema>,
    config: Arc<EngineConfig>,
) -> Router {
    let mut router = Router::new();
    let state = ApiState {
        executor,
        schema: schema.clone(),
        config,
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
    request: Request,
) -> Result<Response, ApiError> {
    let path_pattern = matched_path
        .as_ref()
        .map(|p| p.as_str())
        .ok_or_else(|| ApiError::Internal("No matched path".into()))?;

    debug!("Request received: {} {}", method, path_pattern);

    let api = state
        .schema
        .apis
        .iter()
        .find(|api| {
            let normalized_path = normalize_path(&api.path);
            normalized_path == path_pattern && method_matches(&api.method, &method)
        })
        .ok_or_else(|| {
            ApiError::NotFound(format!("No handler found for {} {}", method, path_pattern))
        })?;

    let handler_name = match state.config.language {
        config::Language::TypeScript => api.name.clone(),
        config::Language::Python => templates::to_snake_case(api.name.clone().as_str()),
    };

    let api_path = api.path.clone();
    debug!("Matched handler: {}", handler_name);

    let normalized_api_path = normalize_path(&api_path);
    let path_params = extract_path_params(&normalized_api_path, request.uri().path());

    let query_params = request
        .uri()
        .query()
        .map(|q| parse_query_string(q))
        .unwrap_or_default();

    let body_bytes = axum::body::to_bytes(request.into_body(), usize::MAX)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to read body: {}", e)))?;

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

    execute_handler(state, handler_name, payload, query_params).await
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
) -> Result<Response, ApiError> {
    let result = state
        .executor
        .execute_with_params(&handler_name, payload, query_params)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    if result.success {
        Ok((StatusCode::OK, Json(result.data.unwrap_or(Value::Null))).into_response())
    } else {
        Ok((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": result.error.unwrap_or_else(|| "Unknown error".to_string()),
            })),
        )
            .into_response())
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
