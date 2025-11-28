use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

use crate::config::EngineConfig;
 
#[derive(Clone, Debug)]
pub struct WorkbenchAuthConfig {
    pub api_key: Option<String>,
    pub allowed_origins: Vec<String>,
}

impl Default for WorkbenchAuthConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            allowed_origins: vec![],
        }
    }
}

impl WorkbenchAuthConfig {
    pub fn from_engine_config(config: &EngineConfig) -> Self {
        let env_key = std::env::var("ROHAS_WORKBENCH_API_KEY")
            .ok()
            .filter(|k| !k.is_empty());

        let env_origins = std::env::var("ROHAS_WORKBENCH_ALLOWED_ORIGINS")
            .ok()
            .map(|origins| {
                origins
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<String>>()
            });

        let api_key = env_key.or_else(|| Some(config.workbench.api_key.clone()));
        let allowed_origins = env_origins.unwrap_or_else(|| config.workbench.allowed_origins.clone());

        Self {
            api_key,
            allowed_origins,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.api_key.is_some()
    }
}

pub async fn workbench_auth_middleware(
    request: Request,
    next: Next,
    config: Arc<tokio::sync::RwLock<WorkbenchAuthConfig>>,
) -> Result<Response, StatusCode> {
    let config = config.read().await;
    let headers = request.headers();

    if !config.is_enabled() {
        return Ok(next.run(request).await);
    }

    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let provided_key = if auth_header.starts_with("Bearer ") {
        auth_header.strip_prefix("Bearer ").unwrap_or("")
    } else if auth_header.starts_with("ApiKey ") {
        auth_header.strip_prefix("ApiKey ").unwrap_or("")
    } else {
        headers
            .get("x-api-key")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("")
    };

    if let Some(ref expected_key) = config.api_key {
        if provided_key != expected_key {
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    if !config.allowed_origins.is_empty() {
        let origin = headers
            .get("origin")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        if !origin.is_empty() && !config.allowed_origins.contains(&origin.to_string()) {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    Ok(next.run(request).await)
}

