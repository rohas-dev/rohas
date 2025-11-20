use crate::error::Result;
use crate::handler::{HandlerContext, HandlerResult};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::{debug, info};
use v8::{Context, ContextScope, HandleScope, Script};

static V8_PLATFORM: Lazy<()> = Lazy::new(|| {
    let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();
    info!("V8 platform initialized");
});

pub struct NodeRuntime {
    /// Loaded modules cache
    modules: Arc<Mutex<HashMap<String, String>>>,
    /// Project root for resolving compiled output
    project_root: Option<PathBuf>,
}

impl NodeRuntime {
    pub fn set_project_root(&mut self, root: PathBuf) {
        self.project_root = Some(root);
    }

    fn resolve_handler_path(&self, handler_path: &Path) -> PathBuf {
        if let Some(project_root) = &self.project_root {
            let relative_path = if handler_path.is_absolute() {
                handler_path.strip_prefix(project_root).ok()
            } else {
                Some(handler_path)
            };

            if let Some(rel_path) = relative_path {
                if let Some(ext) = rel_path.extension() {
                    if ext == "ts" || ext == "tsx" {
                        let stripped = rel_path.strip_prefix("src").unwrap_or(rel_path);
                        let mut compiled_path = project_root.join(".rohas").join(stripped);
                        compiled_path.set_extension("js");

                        if compiled_path.exists() {
                            debug!("Resolved to compiled path: {:?}", compiled_path);
                            return compiled_path;
                        }
                    }
                }
            }
        }

        handler_path.to_path_buf()
    }

    pub fn new() -> Result<Self> {
        Lazy::force(&V8_PLATFORM);

        info!("V8 JavaScript runtime initialized");

        Ok(Self {
            modules: Arc::new(Mutex::new(HashMap::new())),
            project_root: None,
        })
    }

    pub async fn execute_handler(
        &self,
        handler_path: &Path,
        context: HandlerContext,
    ) -> Result<HandlerResult> {
        let start = std::time::Instant::now();

        debug!("Executing JavaScript handler with V8: {:?}", handler_path);

        let resolved_path = self.resolve_handler_path(handler_path);

        debug!("Resolved handler path: {:?}", resolved_path);

        let absolute_path = if resolved_path.is_absolute() {
            resolved_path
        } else {
            std::env::current_dir()?.join(&resolved_path)
        };

        let handler_code = tokio::fs::read_to_string(&absolute_path).await?;

        let module_key = absolute_path.to_string_lossy().to_string();
        {
            let mut modules = self.modules.lock().unwrap();
            modules.insert(module_key.clone(), handler_code.clone());
        }

        let result = tokio::task::spawn_blocking(move || {
            Self::execute_js_code_sync(&handler_code, &context)
        })
        .await
        .map_err(|e| {
            crate::error::RuntimeError::ExecutionFailed(format!("Blocking task failed: {}", e))
        })??;

        let execution_time_ms = start.elapsed().as_millis() as u64;
        Ok(HandlerResult {
            execution_time_ms,
            ..result
        })
    }

    fn execute_js_code_sync(handler_code: &str, context: &HandlerContext) -> Result<HandlerResult> {
        let context_json = serde_json::to_string(context)?;

        let wrapper = Self::generate_wrapper(handler_code, &context_json);

        let isolate = &mut v8::Isolate::new(v8::CreateParams::default());
        let scope = std::pin::pin!(HandleScope::new(isolate));
        let scope = &mut scope.init();
        let v8_context = Context::new(scope, Default::default());
        let scope = &mut ContextScope::new(scope, v8_context);

        let code = v8::String::new(scope, &wrapper).ok_or_else(|| {
            crate::error::RuntimeError::ExecutionFailed("Failed to create V8 string".into())
        })?;

        let script = Script::compile(scope, code, None).ok_or_else(|| {
            crate::error::RuntimeError::ExecutionFailed("Failed to compile script".into())
        })?;

        let result = script.run(scope).ok_or_else(|| {
            crate::error::RuntimeError::ExecutionFailed("Script execution failed".into())
        })?;

        let result = if result.is_promise() {
            let promise = v8::Local::<v8::Promise>::try_from(result).map_err(|_| {
                crate::error::RuntimeError::ExecutionFailed("Failed to cast to Promise".into())
            })?;

            while promise.state() == v8::PromiseState::Pending {
                scope.perform_microtask_checkpoint();
            }

            if promise.state() == v8::PromiseState::Fulfilled {
                promise.result(scope)
            } else {
                let exception = promise.result(scope);
                let error_msg = exception
                    .to_string(scope)
                    .unwrap()
                    .to_rust_string_lossy(scope);
                return Ok(HandlerResult {
                    success: false,
                    data: None,
                    error: Some(error_msg),
                    execution_time_ms: 0,
                });
            }
        } else {
            result
        };

        let json_result = v8::json::stringify(scope, result).ok_or_else(|| {
            crate::error::RuntimeError::ExecutionFailed("Failed to stringify result".into())
        })?;

        let result_str = json_result.to_rust_string_lossy(scope);

        let handler_result: HandlerResult = serde_json::from_str(&result_str)?;

        Ok(handler_result)
    }

    fn generate_wrapper(handler_code: &str, context_json: &str) -> String {
        let context_escaped = context_json
            .replace('\\', "\\\\")
            .replace('\'', "\\'")
            .replace('\n', "\\n")
            .replace('\r', "\\r");

        format!(
            r#"
(async () => {{
    try {{
        // CommonJS shim for V8
        const module = {{ exports: {{}} }};
        const exports = module.exports;
        const require = function(id) {{
            throw new Error('require() is not supported in V8 runtime: ' + id);
        }};

        // Load handler code (CommonJS or plain)
        (function(exports, module, require) {{
            {}
        }})(exports, module, require);

        // Parse context
        const context = JSON.parse('{}');

        // Find handler function
        let handlerFn;

        // Try CommonJS exports - check if module.exports is directly a function
        if (typeof module.exports === 'function') {{
            handlerFn = module.exports;
        }} else if (module.exports && typeof module.exports === 'object') {{
            // Try CommonJS exports (exports.handler or exports.handleXxx)
            const exportKeys = Object.keys(module.exports);

            // Look for any exported function (handleXxx, handler, default)
            for (const key of exportKeys) {{
                if (typeof module.exports[key] === 'function') {{
                    handlerFn = module.exports[key];
                    break;
                }}
            }}
        }}

        // Fallback to global handler function
        if (!handlerFn && typeof handler !== 'undefined') {{
            handlerFn = handler;
        }}

        if (!handlerFn) {{
            throw new Error('Handler not found: No exported function or global handler');
        }}

        // Execute handler
        const result = await handlerFn(context);

        // Return success result
        return {{
            success: true,
            data: result,
            error: null,
            execution_time_ms: 0
        }};
    }} catch (error) {{
        // Return error result
        return {{
            success: false,
            data: null,
            error: error.message + '\n' + (error.stack || ''),
            execution_time_ms: 0
        }};
    }}
}})()
"#,
            handler_code, context_escaped
        )
    }

    pub async fn load_module(&self, module_path: &Path) -> Result<()> {
        info!("Loading JavaScript module: {:?}", module_path);

        let absolute_path = if module_path.is_absolute() {
            module_path.to_path_buf()
        } else {
            std::env::current_dir()?.join(module_path)
        };

        let code = tokio::fs::read_to_string(&absolute_path).await?;
        let module_key = absolute_path.to_string_lossy().to_string();

        let mut modules = self.modules.lock().unwrap();
        modules.insert(module_key.clone(), code);

        info!("Module loaded: {}", module_key);
        Ok(())
    }

    pub async fn reload_module(&self, module_name: &str) -> Result<()> {
        let mut modules = self.modules.lock().unwrap();
        modules.remove(module_name);
        info!("Reloaded module: {}", module_name);
        Ok(())
    }

    pub async fn clear_cache(&self) -> Result<()> {
        let mut modules = self.modules.lock().unwrap();
        modules.clear();
        info!("Cleared all cached modules");
        Ok(())
    }

    pub async fn get_loaded_modules(&self) -> Vec<String> {
        let modules = self.modules.lock().unwrap();
        modules.keys().cloned().collect()
    }
}

impl Default for NodeRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to initialize V8 runtime")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_node_runtime_creation() {
        let runtime = NodeRuntime::new();
        assert!(runtime.is_ok());
    }

    #[tokio::test]
    async fn test_simple_handler_execution() {
        let _runtime = NodeRuntime::new().unwrap();

        let handler_code = r#"
            module.exports = async function handler(context) {
                return { message: "Hello from V8", payload: context.payload };
            };
        "#;

        let context = HandlerContext::new("test", serde_json::json!({"data": "test"}));

        let result = NodeRuntime::execute_js_code_sync(handler_code, &context);
        assert!(result.is_ok());

        let result = result.unwrap();
        assert!(result.success);
        assert!(result.data.is_some());
    }
}
