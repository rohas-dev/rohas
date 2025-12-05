use crate::error::{Result, RuntimeError};
use crate::handler::{HandlerContext, HandlerResult};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

pub struct RustRuntime {
    handlers: Arc<RwLock<HashMap<String, RustHandlerFn>>>,
    project_root: Arc<Mutex<Option<PathBuf>>>,
}

type RustHandlerFn = Box<
    dyn Fn(HandlerContext) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<HandlerResult>> + Send>> + Send + Sync,
>;

impl RustRuntime {
    pub fn new() -> Result<Self> {
        info!("Rust runtime initialized");

        Ok(Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            project_root: Arc::new(Mutex::new(None)),
        })
    }

    pub fn set_project_root(&mut self, root: PathBuf) {
        let mut project_root = self.project_root.lock().unwrap();
        *project_root = Some(root);
    }

    pub async fn register_handler<F, Fut>(&self, name: String, handler: F)
    where
        F: Fn(HandlerContext) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<HandlerResult>> + Send + 'static,
    {
        let handler_fn: RustHandlerFn = Box::new(move |ctx| {
            Box::pin(handler(ctx))
        });

        let mut handlers = self.handlers.write().await;
        let was_present = handlers.contains_key(&name);

        let handler_ptr = &handler_fn as *const _ as usize;
        if was_present {
            if let Some(old_handler) = handlers.get(&name) {
                let old_ptr = old_handler as *const _ as usize;
                info!("Re-registering Rust handler: {} (old closure ptr: 0x{:x}, new closure ptr: 0x{:x})", name, old_ptr, handler_ptr);
                if old_ptr == handler_ptr {
                    warn!("WARNING: Handler closure pointer is the same! This may indicate the handler wasn't actually replaced.");
                } else {
                    info!("Handler closure pointer changed - old handler should be replaced");
                }
            } else {
                info!("Re-registering Rust handler: {} (new closure ptr: 0x{:x})", name, handler_ptr);
            }
            handlers.remove(&name);
        } else {
            info!("Registering Rust handler: {} (closure ptr: 0x{:x})", name, handler_ptr);
        }

        handlers.insert(name.clone(), handler_fn);
    }

    pub async fn execute_handler(
        &self,
        handler_path: &Path,
        context: HandlerContext,
    ) -> Result<HandlerResult> {
        let start = std::time::Instant::now();
        let handler_name = context.handler_name.clone();

        debug!("Executing Rust handler: {:?}", handler_path);

        {
            let handlers = self.handlers.read().await;
            if let Some(handler_fn) = handlers.get(&handler_name) {
                let closure_ptr = handler_fn as *const _ as usize;
                debug!("Executing registered Rust handler: {} (closure pointer: 0x{:x})", handler_name, closure_ptr);
                let result = handler_fn(context).await?;
                let execution_time_ms = start.elapsed().as_millis() as u64;

                if let Some(data) = &result.data {
                    if let Some(data_str) = data.as_str() {
                        debug!("Handler response snippet: {}...", &data_str[..data_str.len().min(50)]);
                    }
                }

                return Ok(HandlerResult {
                    execution_time_ms,
                    ..result
                });
            }
        }

        self.execute_handler_from_file(handler_path, context, start).await
    }

    async fn execute_handler_from_file(
        &self,
        handler_path: &Path,
        context: HandlerContext,
        start: std::time::Instant,
    ) -> Result<HandlerResult> {
        let execution_time_ms = start.elapsed().as_millis() as u64;

        Err(RuntimeError::HandlerNotFound(format!(
            "Rust handler '{}' not found. To register handlers, call the init_handlers function from your generated project.\n\
            Example: rust_example::init_handlers(runtime).await\n\
            Or call: generated::register_all_handlers(runtime).await\n\
            Handler path: {:?}",
            context.handler_name,
            handler_path
        )))
    }

    pub async fn handler_count(&self) -> usize {
        let handlers = self.handlers.read().await;
        handlers.len()
    }

    pub async fn list_handlers(&self) -> Vec<String> {
        let handlers = self.handlers.read().await;
        handlers.keys().cloned().collect()
    }

    pub async fn clear_handlers(&self) {
        let mut handlers = self.handlers.write().await;
        handlers.clear();
        info!("Cleared all Rust handlers");
    }
}

impl Default for RustRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to initialize Rust runtime")
    }
}

/// Helper macro for registering Rust handlers.
///
/// This macro simplifies handler registration and ensures type safety.
///
/// # Example
/// ```rust
/// use rohas_runtime::{rust_runtime, HandlerContext, HandlerResult};
///
/// async fn my_handler(ctx: HandlerContext) -> Result<HandlerResult> {
///     // Handler implementation
///     Ok(HandlerResult::success(serde_json::json!({}), 0))
/// }
///
/// // Register the handler
/// runtime.register_handler("my_handler".to_string(), my_handler).await;
/// ```
#[macro_export]
macro_rules! register_rust_handler {
    ($runtime:expr, $name:expr, $handler:expr) => {
        $runtime.register_handler($name.to_string(), $handler).await
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handler::HandlerContext;

    #[tokio::test]
    async fn test_rust_runtime_creation() {
        let runtime = RustRuntime::new().unwrap();
        assert_eq!(runtime.handler_count().await, 0);
    }

    #[tokio::test]
    async fn test_handler_registration() {
        let runtime = Arc::new(RustRuntime::new().unwrap());

        let handler = |ctx: HandlerContext| async move {
            Ok(HandlerResult::success(
                serde_json::json!({"message": "test"}),
                0,
            ))
        };

        runtime
            .register_handler("test_handler".to_string(), handler)
            .await;

        assert_eq!(runtime.handler_count().await, 1);

        let handlers = runtime.list_handlers().await;
        assert_eq!(handlers, vec!["test_handler"]);
    }

    #[tokio::test]
    async fn test_handler_execution() {
        let runtime = Arc::new(RustRuntime::new().unwrap());

        let handler = |ctx: HandlerContext| async move {
            Ok(HandlerResult::success(
                serde_json::json!({
                    "handler": ctx.handler_name,
                    "message": "executed"
                }),
                0,
            ))
        };

        runtime
            .register_handler("test_exec".to_string(), handler)
            .await;

        let context = HandlerContext::new("test_exec", serde_json::json!({}));
        let result = runtime
            .execute_handler(Path::new("test.rs"), context)
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(
            result.data.unwrap()["handler"],
            serde_json::json!("test_exec")
        );
    }
}

