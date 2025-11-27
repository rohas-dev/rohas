use crate::error::{Result, RuntimeError};
use crate::handler::{HandlerContext, HandlerResult};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};
use rohas_codegen::templates;
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

        let is_event_handler = handler_path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|n| n == "events")
            .unwrap_or(false);

        let is_websocket_handler = handler_path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|n| n == "websockets")
            .unwrap_or(false);

        let function_name = if is_event_handler || is_websocket_handler {
            handler_name.to_string()
        } else {
            Self::extract_function_name(handler_name)
        };

        let handler_fn = module.getattr(function_name.as_str()).map_err(|e| {
            RuntimeError::HandlerNotFound(format!("Function '{}' not found: {}", function_name, e))
        })?;

        let inspect = py.import("inspect")?;
        let sig = inspect.call_method1("signature", (handler_fn.as_any(),))?;
        let params = sig.getattr("parameters")?;
        let param_count = params.call_method0("__len__")?.extract::<usize>()?;

        let state_module = py.import("generated.state")?;
        let state_class = state_module.getattr("State")?;
        let state_obj = state_class.call0()?;
        let state_obj_for_triggers = state_obj.clone();

        let result = if param_count == 0 {
            handler_fn
                .call0()
                .map_err(|e| RuntimeError::ExecutionFailed(format!("Handler call failed: {}", e)))?
        } else if is_event_handler {
            let event_obj = Self::instantiate_event_object(py, context, &handler_path)
                .map_err(|e| RuntimeError::ExecutionFailed(format!("Failed to instantiate event object: {}", e)))?;

            if param_count >= 2 {
                handler_fn
                    .call1((event_obj, state_obj))
                    .map_err(|e| RuntimeError::ExecutionFailed(format!("Handler call failed: {}", e)))?
            } else {
                handler_fn
                    .call1((event_obj,))
                    .map_err(|e| RuntimeError::ExecutionFailed(format!("Handler call failed: {}", e)))?
            }
        } else if is_websocket_handler {
            Self::call_websocket_handler(py, handler_fn, context, param_count, state_obj)
                .map_err(|e| RuntimeError::ExecutionFailed(format!("Handler call failed: {}", e)))?
        } else if param_count >= 2 {
            let request_dict = Self::build_request_dict(py, context)?;
            let request_obj = Self::instantiate_request_class(py, handler_name, &request_dict)
                .unwrap_or_else(|_| request_dict.clone().into_any());

            handler_fn
                .call1((request_obj, state_obj))
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
            let json_module = py.import("json")?;

            let json_ready = if let Ok(model_dump) = final_result.getattr("model_dump") {  
                match model_dump.call0() {
                    Ok(dumped) => dumped,
                    Err(_) => final_result,
                }
            } else if let Ok(dict_method) = final_result.getattr("dict") {
         
                match dict_method.call0() {
                    Ok(dumped) => dumped,
                    Err(_) => final_result,
                }
            } else {
                let dataclasses = py.import("dataclasses")?;
                if dataclasses
                    .call_method1("is_dataclass", (final_result.as_any(),))
                    .and_then(|r| r.extract::<bool>())
                    .unwrap_or(false)
                {
                    match dataclasses.call_method1("asdict", (final_result.as_any(),)) {
                        Ok(dict) => dict,
                        Err(_) => final_result,
                    }
                } else {
                    final_result
                }
            };

            match json_module.call_method1("dumps", (json_ready.as_any(),)) {
                Ok(json_result) => json_result.extract::<String>()?,
                Err(e) => {
                    debug!("Failed to serialize response to JSON: {}, falling back to string representation", e);
                    json_ready.str()?.to_string()
                }
            }
        };

        let data: serde_json::Value =
            serde_json::from_str(&json_str).unwrap_or(serde_json::json!({"raw": json_str}));

        let mut result = HandlerResult::success(data, 0);
        if param_count >= 2 || (is_event_handler && param_count >= 2) {
            if let Ok(triggers_py) = state_obj_for_triggers.call_method0("get_triggers") {
                if let Ok(triggers_list) = triggers_py.downcast::<pyo3::types::PyList>() {
                    for trigger_item in triggers_list.iter() {
                        let event_name_py = trigger_item.getattr("event_name");
                        let payload_py = trigger_item.getattr("payload");
                        
                        if let (Ok(event_name_py), Ok(payload_py)) = (event_name_py, payload_py) {
                            if let Ok(event_name) = event_name_py.extract::<String>() {
                                let json_module = py.import("json")?;
                                if let Ok(payload_str) = json_module.call_method1("dumps", (payload_py,))?.extract::<String>() {
                                    if let Ok(payload_value) = serde_json::from_str::<serde_json::Value>(&payload_str) {
                                        debug!("Extracted manual trigger: {} with payload", event_name);
                                        result = result.with_trigger(event_name, payload_value);
                                    } else {
                                        debug!("Failed to parse payload JSON for trigger: {}", event_name);
                                    }
                                } else {
                                    debug!("Failed to serialize payload to JSON for trigger: {}", event_name);
                                }
                            } else {
                                debug!("Failed to extract event_name from TriggeredEvent");
                            }
                        } else {
                            debug!("Failed to get event_name or payload from TriggeredEvent");
                        }
                    }
                } else {
                    debug!("get_triggers() did not return a list");
                }
            } else {
                debug!("Failed to call get_triggers() on State object");
            }
            
            if let Ok(payloads_py) = state_obj_for_triggers.call_method0("get_all_auto_trigger_payloads") {
                if let Ok(payloads_dict) = payloads_py.downcast::<PyDict>() {
                    for item in payloads_dict.iter() {
                        let key = item.0;
                        let value = item.1;
                        if let Ok(event_name) = key.extract::<String>() {
                            let json_module = py.import("json")?;
                            if let Ok(payload_str) = json_module.call_method1("dumps", (value,))?.extract::<String>() {
                                if let Ok(payload_value) = serde_json::from_str::<serde_json::Value>(&payload_str) {
                                    result = result.with_auto_trigger_payload(event_name, payload_value);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(result)
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

    fn is_primitive_type(type_name: &str) -> bool {
        matches!(
            type_name,
            "String" | "Int" | "Float" | "Boolean" | "Bool" | "DateTime" | "Date"
        )
    }

    fn extract_primitive_value<'py>(
        py: Python<'py>,
        payload_dict: &Bound<'py, pyo3::PyAny>,
        payload_type: &str,
    ) -> PyResult<Bound<'py, pyo3::PyAny>> {

        if let Ok(payload_dict_ref) = payload_dict.downcast::<PyDict>() {
            if let Ok(Some(value)) = payload_dict_ref.get_item("payload") {
                debug!("Extracted primitive value from payload dict for type: {}", payload_type);
                return Ok(value);
            }
            let len = payload_dict_ref.len();
            if len == 1 {
                if let Some((_, value)) = payload_dict_ref.iter().next() {
                    debug!("Extracted primitive value from single-key dict for type: {}", payload_type);
                    return Ok(value);
                }
            }
        }
        Ok(payload_dict.clone())
    }

    fn instantiate_event_object<'py>(
        py: Python<'py>,
        context: &HandlerContext,
        _handler_path: &Path,
    ) -> PyResult<Bound<'py, pyo3::PyAny>> {
        let event_name = context.metadata
            .get("event_name")
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("Event name not found in context metadata"))?;
        let payload_type = context.metadata.get("event_payload_type");

        let json_str = serde_json::to_string(&context.payload)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        let json_module = py.import("json")?;
        let payload_dict = json_module.call_method1("loads", (json_str,))?;
        let payload_dict_clone = payload_dict.clone();

        let datetime_module = py.import("datetime")?;
        let now = datetime_module.getattr("datetime")?.getattr("now")?.call0()?;
        let now_clone = now.clone();

        let convert_snake_to_camel = |dict: &Bound<'_, PyDict>| -> PyResult<Bound<'_, PyDict>> {
            let camel_dict = PyDict::new(py);
            for (key, value) in dict.iter() {
                if let Ok(key_str) = key.extract::<String>() {
                    let camel_key = if key_str.contains('_') {
                        let parts: Vec<&str> = key_str.split('_').collect();
                        let mut camel = String::new();
                        for (i, part) in parts.iter().enumerate() {
                            if i == 0 {
                                camel.push_str(part);
                            } else {
                                let mut chars = part.chars();
                                if let Some(first) = chars.next() {
                                    camel.push(first.to_uppercase().next().unwrap());
                                    camel.push_str(&chars.as_str());
                                }
                            }
                        }
                        camel
                    } else {
                        key_str
                    };
                    camel_dict.set_item(camel_key, value)?;
                } else {
                    camel_dict.set_item(key, value)?;
                }
            }
            Ok(camel_dict)
        };

        let payload_obj = if let Some(payload_type_name) = payload_type {
            if Self::is_primitive_type(payload_type_name) {
                debug!("Payload type {} is primitive, extracting value", payload_type_name);
                Self::extract_primitive_value(py, &payload_dict, payload_type_name)?
            } else {
                let payload_type_snake = templates::to_snake_case(payload_type_name);
                let model_module_path = format!("generated.models.{}", payload_type_snake);
                
                match py.import(&model_module_path) {
                    Ok(model_module) => {
                        match model_module.getattr(payload_type_name.as_str()) {
                            Ok(model_class) => {
                                if let Ok(payload_dict_ref) = payload_dict.downcast::<PyDict>() {
                                    match convert_snake_to_camel(&payload_dict_ref) {
                                        Ok(camel_payload_dict) => {
                                            match model_class.call((), Some(&camel_payload_dict)) {
                                            Ok(model_obj) => {
                                                debug!("Successfully instantiated payload model: {}", payload_type_name);
                                                model_obj.into_any()
                                            }
                                            Err(e) => {
                                                debug!("Direct call failed for {}, trying model_validate: {}", payload_type_name, e);
                                                if let Ok(model_validate) = model_class.getattr("model_validate") {
                                                    match model_validate.call1((camel_payload_dict,)) {
                                                        Ok(model_obj) => {
                                                            debug!("Successfully instantiated payload model via model_validate: {}", payload_type_name);
                                                            model_obj.into_any()
                                                        }
                                                        Err(e2) => {
                                                            debug!("model_validate also failed for {}: {}", payload_type_name, e2);
                                                            payload_dict.clone()
                                                        }
                                                    }
                                                } else {
                                                    payload_dict.clone()
                                                }
                                            }
                                        }
                                        }
                                        Err(e) => {
                                            debug!("Failed to convert field names: {}, trying with original dict", e);
                                            match model_class.call((), Some(&payload_dict_ref)) {
                                                Ok(model_obj) => {
                                                    debug!("Successfully instantiated payload model with original dict: {}", payload_type_name);
                                                    model_obj.into_any()
                                                }
                                                Err(_) => {
                                                    payload_dict.clone()
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    if let Ok(model_validate) = model_class.getattr("model_validate") {
                                        match model_validate.call1((payload_dict.clone(),)) {
                                            Ok(model_obj) => {
                                                debug!("Successfully instantiated payload model via model_validate (PyAny): {}", payload_type_name);
                                                model_obj.into_any()
                                            }
                                            Err(e) => {
                                                debug!("model_validate failed for {} (PyAny): {}", payload_type_name, e);
                                                payload_dict.clone()
                                            }
                                        }
                                    } else {
                                        payload_dict.clone()
                                    }
                                }
                            }
                            Err(e) => {
                                debug!("Failed to get model class {} from module: {}", payload_type_name, e);
                                payload_dict.clone()
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Failed to import model module {}: {}", model_module_path, e);
                        payload_dict.clone()
                    }
                }
            }
        } else {
            debug!("No payload type in metadata, using dict as-is");
            payload_dict.clone()
        };

        let event_name_snake = templates::to_snake_case(event_name);
        let event_module_path = format!("generated.events.{}", event_name_snake);
        
        match py.import(&event_module_path) {
            Ok(event_module) => {
                match event_module.getattr(event_name.as_str()) {
                    Ok(event_class) => {
                        let event_dict = PyDict::new(py);
                        event_dict.set_item("payload", payload_obj)?;
                        event_dict.set_item("timestamp", &now)?;
                        
                        let mut event_dict_for_direct = None;
                        if let Ok(model_validate) = event_class.getattr("model_validate") {
                            debug!("Attempting model_validate for event {}", event_name);
                            match model_validate.call1((event_dict,)) {
                                Ok(event_obj) => {
                                    debug!("model_validate call succeeded for event {}", event_name);
                                    match event_obj.getattr("payload") {
                                        Ok(_) => {
                                            debug!("Event object has payload attribute - instantiation successful via model_validate");
                                            return Ok(event_obj);
                                        }
                                        Err(e) => {
                                            debug!("Event object from model_validate missing payload attribute: {}", e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    let error_msg = format!("{}", e);
                                    debug!("model_validate failed for event {}: {}", event_name, error_msg);
                                    let py_err = e.value(py);
                                    if let Ok(err_str) = py_err.str() {
                                        debug!("Python error details: {}", err_str.to_string_lossy());
                                    }
                                    let json_str_direct = serde_json::to_string(&context.payload)
                                        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
                                    let payload_for_direct = json_module.call_method1("loads", (json_str_direct,))?;
                                    let payload_for_direct_value = if let Some(payload_type_name) = payload_type {
                                        if Self::is_primitive_type(payload_type_name) {
                                            Self::extract_primitive_value(py, &payload_for_direct, payload_type_name)?
                                        } else {
                                            payload_for_direct
                                        }
                                    } else {
                                        payload_for_direct
                                    };
                                    let event_dict2 = PyDict::new(py);
                                    event_dict2.set_item("payload", payload_for_direct_value)?;
                                    event_dict2.set_item("timestamp", &now_clone)?;
                                    event_dict_for_direct = Some(event_dict2);
                                }
                            }
                        } else {
                            debug!("model_validate method not found for event {}, using direct call", event_name);
                            event_dict_for_direct = Some(event_dict);
                        }
                        
                        if let Some(event_dict2) = event_dict_for_direct {
                            debug!("Attempting direct call for event {}", event_name);
                            match event_class.call((), Some(&event_dict2)) {
                                Ok(event_obj) => {
                                    debug!("Direct call succeeded for event {}", event_name);
                                    match event_obj.getattr("payload") {
                                        Ok(_) => {
                                            debug!("Event object has payload attribute - instantiation successful via direct call");
                                            return Ok(event_obj);
                                        }
                                        Err(e) => {
                                            debug!("Event object missing payload attribute: {}", e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    let error_msg = format!("{}", e);
                                    debug!("Direct call also failed for event {}, error: {}", event_name, error_msg);
                                    let py_err = e.value(py);
                                    if let Ok(err_str) = py_err.str() {
                                        debug!("Python error details: {}", err_str.to_string_lossy());
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Failed to get event class {} from module: {}", event_name, e);
                    }
                }
            }
            Err(e) => {
                debug!("Failed to import event module {}: {}", event_module_path, e);
            }
        }

        debug!("Attempting final fallback instantiation for event: {} with dict payload", event_name);
        let final_payload_value = if let Some(payload_type_name) = payload_type {
            if Self::is_primitive_type(payload_type_name) {
                Self::extract_primitive_value(py, &payload_dict_clone, payload_type_name)?
            } else {
                payload_dict_clone
            }
        } else {
            payload_dict_clone
        };
        let final_event_dict = PyDict::new(py);
        final_event_dict.set_item("payload", final_payload_value)?;
        final_event_dict.set_item("timestamp", &now_clone)?;
        
        if let Ok(event_module) = py.import(&event_module_path) {
            if let Ok(event_class) = event_module.getattr(event_name.as_str()) {
                let mut final_dict_for_direct = None;
                if let Ok(model_validate) = event_class.getattr("model_validate") {
                    match model_validate.call1((final_event_dict,)) {
                        Ok(event_obj) => {
                            if event_obj.getattr("payload").is_ok() {
                                debug!("Successfully instantiated event via fallback model_validate with dict payload");
                                return Ok(event_obj);
                            }
                        }
                        Err(e) => {
                            debug!("Fallback model_validate failed: {}", e);
                            let json_str_fallback2 = serde_json::to_string(&context.payload)
                                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
                            let payload_dict_fallback2 = json_module.call_method1("loads", (json_str_fallback2,))?;
                            let payload_fallback2_value = if let Some(payload_type_name) = payload_type {
                                if Self::is_primitive_type(payload_type_name) {
                                    Self::extract_primitive_value(py, &payload_dict_fallback2, payload_type_name)?
                                } else {
                                    payload_dict_fallback2
                                }
                            } else {
                                payload_dict_fallback2
                            };
                            let final_dict2 = PyDict::new(py);
                            final_dict2.set_item("payload", payload_fallback2_value)?;
                            final_dict2.set_item("timestamp", &now_clone)?;
                            final_dict_for_direct = Some(final_dict2);
                        }
                    }
                } else {
                    final_dict_for_direct = Some(final_event_dict);
                }
                if let Some(final_dict2) = final_dict_for_direct {
                    match event_class.call((), Some(&final_dict2)) {
                    Ok(event_obj) => {
                        if event_obj.getattr("payload").is_ok() {
                            debug!("Successfully instantiated event via fallback direct call with dict payload");
                            return Ok(event_obj);
                        }
                    }
                    Err(e) => {
                        debug!("Fallback direct call also failed: {}", e);
                    }
                    }
                }
            }
        }
        
        Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Failed to instantiate event object {}: All instantiation methods failed. Check debug logs for details.",
            event_name
        )))
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

    fn call_websocket_handler<'py>(
        py: Python<'py>,
        handler_fn: Bound<'py, pyo3::PyAny>,
        context: &HandlerContext,
        param_count: usize,
        state_obj: Bound<'py, pyo3::PyAny>,
    ) -> PyResult<Bound<'py, pyo3::PyAny>> {
        let json_str = serde_json::to_string(&context.payload)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        let json_module = py.import("json")?;
        let payload_dict = json_module.call_method1("loads", (json_str,))?;
        let payload_dict = payload_dict.downcast::<PyDict>()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Payload is not a dict: {}", e)))?;


        let ws_name = context.metadata
            .get("websocket_name")
            .map(|s| s.as_str())
            .unwrap_or("HelloWorld");
        
        let ws_name_snake = templates::to_snake_case(ws_name);
        let ws_module_path = format!("generated.websockets.{}", ws_name_snake);
 
        let (connection_class, message_class) = match py.import(&ws_module_path) {
            Ok(ws_module) => {
                let conn_class = ws_module.getattr(&format!("{}Connection", ws_name)).ok();
                let msg_class = ws_module.getattr(&format!("{}Message", ws_name)).ok();
                (conn_class, msg_class)
            }
            Err(_) => (None, None)
        };
 
        let connection_obj = if let Some(conn_class) = connection_class {
            if let Ok(connection_dict) = payload_dict.get_item("connection") {
                if let Some(conn_dict) = connection_dict {
                    if let Ok(conn_dict) = conn_dict.downcast::<PyDict>() {
                        if let Ok(model_validate) = conn_class.getattr("model_validate") {
                            if let Ok(conn_obj) = model_validate.call1((conn_dict,)) {
                                conn_obj
                            } else {
                                conn_class.call((), Some(conn_dict)).unwrap_or_else(|_| conn_dict.clone().into_any())
                            }
                        } else {
                            conn_class.call((), Some(conn_dict)).unwrap_or_else(|_| conn_dict.clone().into_any())
                        }
                    } else {
                        conn_dict.clone().into_any()
                    }
                } else {
 
                    if let Ok(model_validate) = conn_class.getattr("model_validate") {
                        if let Ok(conn_obj) = model_validate.call1((payload_dict,)) {
                            conn_obj
                        } else {
                            conn_class.call((), Some(payload_dict)).unwrap_or_else(|_| payload_dict.clone().into_any())
                        }
                    } else {
                        conn_class.call((), Some(payload_dict)).unwrap_or_else(|_| payload_dict.clone().into_any())
                    }
                }
            } else {

                if let Ok(model_validate) = conn_class.getattr("model_validate") {
                    if let Ok(conn_obj) = model_validate.call1((payload_dict,)) {
                        conn_obj
                    } else {
                        conn_class.call((), Some(payload_dict)).unwrap_or_else(|_| payload_dict.clone().into_any())
                    }
                } else {
                    conn_class.call((), Some(payload_dict)).unwrap_or_else(|_| payload_dict.clone().into_any())
                }
            }
        } else {
            payload_dict.clone().into_any()
        };

        let message_obj = if param_count >= 3 {
            if let Some(msg_class) = message_class {
                if let Ok(message_dict) = payload_dict.get_item("message") {
                    if let Some(msg_dict) = message_dict {
                        if let Ok(msg_dict) = msg_dict.downcast::<PyDict>() {
                            if let Ok(model_validate) = msg_class.getattr("model_validate") {
                                if let Ok(msg_obj) = model_validate.call1((msg_dict,)) {
                                    Some(msg_obj)
                                } else {
                                    Some(msg_class.call((), Some(msg_dict)).unwrap_or_else(|_| msg_dict.clone().into_any()))
                                }
                            } else {
                                Some(msg_class.call((), Some(msg_dict)).unwrap_or_else(|_| msg_dict.clone().into_any()))
                            }
                        } else {
                            Some(msg_dict.clone().into_any())
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        if param_count == 3 {
            if let Some(msg) = message_obj {
                handler_fn.call1((msg, connection_obj, state_obj))
            } else {
                handler_fn.call1((payload_dict.clone().into_any(), connection_obj, state_obj))
            }
        } else if param_count == 2 {
            handler_fn.call1((connection_obj, state_obj))
        } else {
            handler_fn.call1((connection_obj,))
        }
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
