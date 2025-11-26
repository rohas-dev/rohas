use crate::error::{Result, RuntimeError};
use crate::handler::{HandlerContext, HandlerResult};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use tracing::{debug, info};

pub struct PythonRuntime {
    modules: Arc<RwLock<std::collections::HashMap<String, Py<PyModule>>>>,
    project_root: Arc<Mutex<Option<PathBuf>>>,
}

impl PythonRuntime {
    pub fn new() -> Result<Self> {
        Python::with_gil(|_| {
            info!("Python runtime initialized");
        });

        Ok(Self {
            modules: Arc::new(RwLock::new(std::collections::HashMap::new())),
            project_root: Arc::new(Mutex::new(None)),
        })
    }

    pub fn set_project_root(&mut self, root: PathBuf) {
        let mut project_root = self.project_root.lock().unwrap();
        *project_root = Some(root);
    }

    pub async fn execute_handler(
        &self,
        handler_path: &Path,
        context: HandlerContext,
    ) -> Result<HandlerResult> {
        let start = std::time::Instant::now();
        let handler_path = handler_path.to_path_buf();
        let handler_name = context.handler_name.clone();
        let project_root = self.project_root.lock().unwrap().clone();

        debug!("Executing Python handler: {:?}", handler_path);

        let task = tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                Self::execute_handler_sync(
                    py,
                    &handler_path,
                    &handler_name,
                    &context,
                    project_root.as_ref(),
                )
            })
        });

        let result = tokio::time::timeout(std::time::Duration::from_secs(30), task)
            .await
            .map_err(|_| RuntimeError::ExecutionFailed("Handler execution timeout (30s)".into()))?
            .map_err(|e| RuntimeError::ExecutionFailed(format!("Task join error: {}", e)))??;

        let execution_time_ms = start.elapsed().as_millis() as u64;
        Ok(HandlerResult {
            execution_time_ms,
            ..result
        })
    }

    fn execute_handler_sync(
        py: Python<'_>,
        handler_path: &Path,
        handler_name: &str,
        context: &HandlerContext,
        project_root: Option<&PathBuf>,
    ) -> Result<HandlerResult> {
        let sys = py.import("sys")?;
        let sys_path = sys.getattr("path")?;

        if let Some(root) = project_root {
            let src_path = root.join("src");
            if src_path.exists() {
                sys_path.call_method1("insert", (0, src_path.to_str().unwrap()))?;
                debug!("Added to sys.path: {:?}", src_path);
            }
        }

        if let Some(parent) = handler_path.parent() {
            sys_path.call_method1("insert", (0, parent.to_str().unwrap()))?;
        }

        let module_name = handler_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| RuntimeError::ExecutionFailed("Invalid module name".into()))?;

        let module = PyModule::import(py, module_name).map_err(|e| {
            RuntimeError::ExecutionFailed(format!("Failed to import module: {}", e))
        })?;

        let function_name = Self::extract_function_name(handler_name);
        let handler_fn = module.getattr(function_name.as_str()).map_err(|e| {
            RuntimeError::HandlerNotFound(format!("Function '{}' not found: {}", function_name, e))
        })?;

        let inspect = py.import("inspect")?;
        let sig = inspect.call_method1("signature", (handler_fn.as_any(),))?;
        let params = sig.getattr("parameters")?;
        let param_count = params.call_method0("__len__")?.extract::<usize>()?;

        let result = if param_count == 0 {
            handler_fn
                .call0()
                .map_err(|e| RuntimeError::ExecutionFailed(format!("Handler call failed: {}", e)))?
        } else {
            let request_dict = Self::build_request_dict(py, context)?;

            let request_obj = Self::instantiate_request_class(py, handler_name, &request_dict)
                .unwrap_or_else(|_| request_dict.clone().into_any());

            handler_fn
                .call1((request_obj,))
                .map_err(|e| RuntimeError::ExecutionFailed(format!("Handler call failed: {}", e)))?
        };

        let final_result = if Self::is_coroutine(py, &result)? {
            debug!("Handler is async, awaiting coroutine");
            Self::await_coroutine(py, result)?
        } else {
            result
        };

        let json_str: String = if final_result.is_none() {
            "null".to_string()
        } else {
            let dataclasses = py.import("dataclasses")?;
            let json_module = py.import("json")?;

            let json_ready = if dataclasses
                .call_method1("is_dataclass", (final_result.as_any(),))?
                .extract::<bool>()?
            {
                dataclasses.call_method1("asdict", (final_result.as_any(),))?
            } else {
                final_result
            };

            match json_module.call_method1("dumps", (json_ready.as_any(),)) {
                Ok(json_result) => json_result.extract::<String>()?,
                Err(_) => json_ready.str()?.to_string(),
            }
        };

        let data: serde_json::Value =
            serde_json::from_str(&json_str).unwrap_or(serde_json::json!({"raw": json_str}));

        Ok(HandlerResult::success(data, 0))
    }

    fn build_request_dict<'py>(
        py: Python<'py>,
        context: &HandlerContext,
    ) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);

        let json_str = serde_json::to_string(&context.payload)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        let json_module = py.import("json")?;
        let payload_py = json_module.call_method1("loads", (json_str,))?;

        if let Ok(payload_dict) = payload_py.downcast::<PyDict>() {
            if !payload_dict.is_empty() {
                if let Ok(body_value) = payload_dict.get_item("body") {
                    if let Some(body) = body_value {
                        dict.set_item("body", body)?;
                    }
                } else {
                    for item in payload_dict.iter() {
                        let key = item.0;
                        let value = item.1;

                        dict.set_item(key, value)?;
                    }
                }
            }
        }

        let query_params_dict = PyDict::new(py);
        for (key, value) in &context.query_params {
            query_params_dict.set_item(key, value)?;
        }
        dict.set_item("query_params", query_params_dict)?;

        Ok(dict)
    }

    fn instantiate_request_class<'py>(
        py: Python<'py>,
        handler_name: &str,
        request_dict: &Bound<'py, PyDict>,
    ) -> PyResult<Bound<'py, pyo3::PyAny>> {
        let class_name = Self::handler_name_to_request_class(handler_name);

        let module_name = handler_name.to_lowercase();

        let import_path = format!("generated.api.{}", module_name);
        let api_module = py.import(import_path.as_str())?;
        let request_class = api_module.getattr(class_name.as_str())?;

        request_class.call((), Some(request_dict))
    }

    fn handler_name_to_request_class(handler_name: &str) -> String {
        let pascal_case = handler_name
            .split('_')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<String>();

        format!("{}Request", pascal_case)
    }

    fn context_to_py_dict<'py>(
        py: Python<'py>,
        context: &HandlerContext,
    ) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);

        dict.set_item("handler_name", &context.handler_name)?;

        let json_str = serde_json::to_string(&context.payload)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        let json_module = py.import("json")?;
        let payload_py = json_module.call_method1("loads", (json_str,))?;
        dict.set_item("payload", payload_py)?;

        let query_params_dict = PyDict::new(py);
        for (key, value) in &context.query_params {
            query_params_dict.set_item(key, value)?;
        }
        dict.set_item("query_params", query_params_dict)?;

        dict.set_item("timestamp", &context.timestamp)?;

        let metadata_dict = PyDict::new(py);
        for (key, value) in &context.metadata {
            metadata_dict.set_item(key, value)?;
        }
        dict.set_item("metadata", metadata_dict)?;

        Ok(dict)
    }

    fn is_coroutine(py: Python<'_>, obj: &Bound<'_, pyo3::PyAny>) -> PyResult<bool> {
        let inspect = py.import("inspect")?;
        let is_coro = inspect.call_method1("iscoroutine", (obj,))?;
        is_coro.extract::<bool>()
    }

    fn await_coroutine<'py>(
        py: Python<'py>,
        coro: Bound<'py, pyo3::PyAny>,
    ) -> PyResult<Bound<'py, pyo3::PyAny>> {
        let asyncio = py.import("asyncio")?;

        let loop_result = asyncio.call_method0("get_event_loop");

        let result = if let Ok(event_loop) = loop_result {
            event_loop.call_method1("run_until_complete", (coro,))?
        } else {
            let new_loop = asyncio.call_method0("new_event_loop")?;
            asyncio.call_method1("set_event_loop", (new_loop.as_any(),))?;
            let result = new_loop.call_method1("run_until_complete", (coro,))?;
            new_loop.call_method0("close")?;
            result
        };

        Ok(result)
    }

    fn extract_function_name(handler_name: &str) -> String {
        if handler_name.chars().any(|c| c.is_uppercase()) {
            let snake = to_snake_case(handler_name);
            format!("handle_{}", snake)
        } else {
            format!("handle_{}", handler_name.to_string())
        }
    }

    pub async fn reload_module(&self, module_name: &str) -> Result<()> {
        let mut modules = self.modules.write().await;
        modules.remove(module_name);
        info!("Reloaded Python module: {}", module_name);
        Ok(())
    }
}

impl Default for PythonRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to initialize Python runtime")
    }
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap());
        } else {
            result.push(ch);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name_extraction() {
        assert_eq!(
            PythonRuntime::extract_function_name("send_welcome_email"),
            "handle_send_welcome_email"
        );
        assert_eq!(
            PythonRuntime::extract_function_name("CreateUser"),
            "handle_create_user"
        );
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("CreateUser"), "create_user");
        assert_eq!(to_snake_case("UserCreated"), "user_created");
    }
}
