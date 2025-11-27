use crate::error::{Result, RuntimeError};
use crate::handler::{Handler, HandlerContext, HandlerResult};
use crate::node_runtime::NodeRuntime;
use crate::python_runtime::PythonRuntime;
use crate::{Language, RuntimeConfig};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use rohas_codegen::templates;
use tokio::sync::RwLock;
use tracing::{debug, info};
 
pub struct Executor {
    config: RuntimeConfig,
    handlers: Arc<RwLock<HashMap<String, Arc<dyn Handler>>>>,
    python_runtime: Arc<PythonRuntime>,
    node_runtime: Arc<NodeRuntime>,
}

impl Executor {
    pub fn new(config: RuntimeConfig) -> Self {
        let mut python_runtime = PythonRuntime::new().expect("Failed to initialize Python runtime");
        python_runtime.set_project_root(config.project_root.clone());
        let python_runtime = Arc::new(python_runtime);

        let mut node_runtime = NodeRuntime::new().expect("Failed to initialize Node.js runtime");
        node_runtime.set_project_root(config.project_root.clone());
        let node_runtime = Arc::new(node_runtime);

        info!("Executor initialized with Python and Node.js runtimes");

        Self {
            config,
            handlers: Arc::new(RwLock::new(HashMap::new())),
            python_runtime,
            node_runtime,
        }
    }

    pub async fn register_handler(&self, handler: Arc<dyn Handler>) {
        let name = handler.name().to_string();
        let mut handlers = self.handlers.write().await;
        handlers.insert(name.clone(), handler);
        info!("Registered handler: {}", name);
    }

    pub async fn execute(
        &self,
        handler_name: &str,
        payload: serde_json::Value,
    ) -> Result<HandlerResult> {
        self.execute_with_params(handler_name, payload, HashMap::new())
            .await
    }

    pub async fn execute_with_params(
        &self,
        handler_name: &str,
        payload: serde_json::Value,
        query_params: HashMap<String, String>,
    ) -> Result<HandlerResult> {
        debug!("Executing handler: {}", handler_name);

        let mut context = HandlerContext::new(handler_name, payload);
        context.query_params = query_params;

        {
            let handlers = self.handlers.read().await;
            if let Some(handler) = handlers.get(handler_name) {
                return handler.execute(context.clone()).await;
            }
        }

        self.execute_external_handler(context).await
    }

    pub async fn execute_with_context(&self, context: HandlerContext) -> Result<HandlerResult> {
        debug!("Executing handler: {}", context.handler_name);

        {
            let handlers = self.handlers.read().await;
            if let Some(handler) = handlers.get(&context.handler_name) {
                return handler.execute(context.clone()).await;
            }
        }

        self.execute_external_handler(context).await
    }

    async fn execute_external_handler(&self, context: HandlerContext) -> Result<HandlerResult> {
        let start = std::time::Instant::now();

        let handler_path = self.resolve_handler_path(&context.handler_name)?;

        let result = match self.config.language {
            Language::TypeScript => self.execute_typescript(&handler_path, &context).await,
            Language::Python => self.execute_python(&handler_path, &context).await,
        };

        let execution_time_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(mut res) => {
                res.execution_time_ms = execution_time_ms;
                Ok(res)
            }
            Err(e) => Ok(HandlerResult::error(e.to_string(), execution_time_ms)),
        }
    }

    fn resolve_handler_path(&self, handler_name: &str) -> Result<PathBuf> {
        let handlers_dir = self.config.project_root.join("src/handlers");

        let snake_case_name = templates::to_snake_case(handler_name);

        let possible_paths = [
            handlers_dir.join(format!(
                "api/{}.{}",
                handler_name,
                self.config.language.file_extension()
            )),
            handlers_dir.join(format!(
                "events/{}.{}",
                handler_name,
                self.config.language.file_extension()
            )),
            handlers_dir.join(format!(
                "websockets/{}.{}",
                handler_name,
                self.config.language.file_extension()
            )),
            handlers_dir.join(format!(
                "cron/{}.{}",
                snake_case_name,
                self.config.language.file_extension()
            )),
            handlers_dir.join(format!(
                "cron/{}.{}",
                handler_name,
                self.config.language.file_extension()
            )),
            handlers_dir.join(format!(
                "{}.{}",
                handler_name,
                self.config.language.file_extension()
            )),
        ];

        for path in &possible_paths {
            if path.exists() {
                return Ok(path.clone());
            }
        }

        Err(RuntimeError::HandlerNotFound(format!(
            "Handler '{}' not found in any handlers directory",
            handler_name
        )))
    }

    async fn execute_typescript(
        &self,
        handler_path: &PathBuf,
        context: &HandlerContext,
    ) -> Result<HandlerResult> {
        debug!(
            "Executing TypeScript handler via Node.js runtime: {:?}",
            handler_path
        );
        self.node_runtime
            .execute_handler(handler_path, context.clone())
            .await
    }

    async fn execute_python(
        &self,
        handler_path: &PathBuf,
        context: &HandlerContext,
    ) -> Result<HandlerResult> {
        debug!("Executing Python handler via pyo3: {:?}", handler_path);
        self.python_runtime
            .execute_handler(handler_path, context.clone())
            .await
    }

    pub async fn list_handlers(&self) -> Vec<String> {
        let handlers = self.handlers.read().await;
        handlers.keys().cloned().collect()
    }

    pub async fn reload_python_module(&self, module_name: &str) -> Result<()> {
        self.python_runtime.reload_module(module_name).await
    }

    pub async fn reload_node_module(&self, module_name: &str) -> Result<()> {
        self.node_runtime.reload_module(module_name).await
    }

    pub async fn clear_handler_cache(&self) -> Result<()> {
        match self.config.language {
            Language::TypeScript => {
                self.node_runtime.clear_cache().await?;
            }
            Language::Python => {
                // @TODO Python runtime doesn't cache modules the same way
                // Module reloading is handled differently in pyo3
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHandler {
        name: String,
    }

    #[async_trait::async_trait]
    impl Handler for TestHandler {
        async fn execute(&self, _context: HandlerContext) -> Result<HandlerResult> {
            Ok(HandlerResult::success(
                serde_json::json!({"message": "test"}),
                0,
            ))
        }

        fn name(&self) -> &str {
            &self.name
        }
    }

    #[tokio::test]
    async fn test_register_and_execute_handler() {
        let config = RuntimeConfig::default();
        let executor = Executor::new(config);

        let handler = Arc::new(TestHandler {
            name: "test_handler".to_string(),
        });

        executor.register_handler(handler).await;

        let handlers = executor.list_handlers().await;
        assert_eq!(handlers.len(), 1);
        assert_eq!(handlers[0], "test_handler");

        let result = executor
            .execute("test_handler", serde_json::json!({}))
            .await
            .unwrap();

        assert!(result.success);
    }
}
