use crate::error::Result;
use crate::templates;
use rohas_parser::{Api, Event, FieldType, Model, Schema, WebSocket};
use std::fs;
use std::path::Path;

pub fn generate_models(schema: &Schema, output_dir: &Path) -> Result<()> {
    let models_dir = output_dir.join("generated/models");

    for model in &schema.models {
        let content = generate_model_content(model);
        let file_name = format!("{}.py", templates::to_snake_case(&model.name));
        fs::write(models_dir.join(file_name), content)?;
    }

    Ok(())
}

fn generate_model_content(model: &Model) -> String {
    let mut content = String::new();

    content.push_str("from pydantic import BaseModel\n");
    content.push_str("from typing import Optional\n");
    content.push_str("from datetime import datetime\n\n");

    content.push_str(&format!("class {}(BaseModel):\n", model.name));

    for field in &model.fields {
        let py_type = field.field_type.to_python();
        let type_hint = if field.optional {
            format!("Optional[{}]", py_type)
        } else {
            py_type
        };
        content.push_str(&format!("    {}: {}\n", field.name, type_hint));
    }

    if model.fields.is_empty() {
        content.push_str("    pass\n");
    }

    content.push_str("\n    class Config:\n");
    content.push_str("        from_attributes = True\n");

    content
}

pub fn generate_dtos(schema: &Schema, output_dir: &Path) -> Result<()> {
    let dto_dir = output_dir.join("generated/dto");

    for input in &schema.inputs {
        let content = generate_model_content(&rohas_parser::Model {
            name: input.name.clone(),
            fields: input.fields.clone(),
            attributes: vec![],
        });
        let file_name = format!("{}.py", templates::to_snake_case(&input.name));
        fs::write(dto_dir.join(file_name), content)?;
    }

    Ok(())
}

pub fn generate_apis(schema: &Schema, output_dir: &Path) -> Result<()> {
    let api_dir = output_dir.join("generated/api");

    for api in &schema.apis {
        let content = generate_api_content(api);
        let file_name = format!("{}.py", templates::to_snake_case(&api.name));
        fs::write(api_dir.join(file_name), content)?;
    }

    let handlers_dir = output_dir.join("handlers/api");
    for api in &schema.apis {
        let file_name = format!("{}.py", templates::to_snake_case(&api.name));
        let handler_path = handlers_dir.join(&file_name);

        if !handler_path.exists() {
            let content = generate_api_handler_stub(api);
            fs::write(handler_path, content)?;
        }
    }

    Ok(())
}

/// Extract path parameters from a path string
/// e.g., "/test/{name}" -> ["name"]
/// e.g., "/users/{id}/posts/{postId}" -> ["id", "postId"]
fn extract_path_params(path: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut in_param = false;
    let mut current_param = String::new();

    for ch in path.chars() {
        match ch {
            '{' => {
                in_param = true;
                current_param.clear();
            }
            '}' => {
                if in_param && !current_param.is_empty() {
                    params.push(current_param.clone());
                }
                in_param = false;
            }
            _ if in_param => {
                current_param.push(ch);
            }
            _ => {}
        }
    }

    params
}

fn generate_api_content(api: &Api) -> String {
    let mut content = String::new();

    content.push_str("from pydantic import BaseModel\n");
    content.push_str("from typing import Callable, Awaitable, Dict, Optional\n");

    let response_field_type = FieldType::from_str(&api.response);
    let response_py_type = response_field_type.to_python();

    let is_custom_type = matches!(response_field_type, FieldType::Custom(_));
    if is_custom_type {
        content.push_str(&format!(
            "from ..models.{} import {}\n",
            templates::to_snake_case(&api.response),
            api.response
        ));
    }

    if let Some(body) = &api.body {
        content.push_str(&format!(
            "from ..dto.{} import {}\n",
            templates::to_snake_case(body),
            body
        ));
    }

    let path_params = extract_path_params(&api.path);

    content.push_str(&format!("\nclass {}Request(BaseModel):\n", api.name));

    for param in &path_params {
        content.push_str(&format!("    {}: str\n", param));
    }

    if let Some(body) = &api.body {
        content.push_str(&format!("    body: {}\n", body));
    }

    content.push_str("    query_params: Dict[str, str] = {}\n");

    if path_params.is_empty() && api.body.is_none() {
        // We still have query_params, so no pass needed
    }

    content.push_str("\n    class Config:\n");
    content.push_str("        from_attributes = True\n");

    content.push_str(&format!("\nclass {}Response(BaseModel):\n", api.name));
    content.push_str(&format!("    data: {}\n", response_py_type));

    content.push_str("\n    class Config:\n");
    content.push_str("        from_attributes = True\n");

    content.push_str(&format!(
        "\n{}Handler = Callable[[{}Request], Awaitable[{}Response]]\n",
        api.name, api.name, api.name
    ));

    content
}

fn generate_api_handler_stub(api: &Api) -> String {
    let mut content = String::new();

    content.push_str(&format!(
        "from generated.api.{} import {}Request, {}Response\n",
        templates::to_snake_case(&api.name),
        api.name,
        api.name
    ));
    content.push_str("from generated.state import State\n\n");

    content.push_str(&format!(
        "async def handle_{}(req: {}Request, state: State) -> {}Response:\n",
        templates::to_snake_case(&api.name),
        api.name,
        api.name
    ));
    content.push_str("    # TODO: Implement handler logic\n");
    content.push_str("    # For auto-triggers (defined in schema triggers): use state.set_payload('EventName', {...})\n");
    content.push_str("    # For manual triggers: use state.trigger_event('EventName', {...})\n");
    content.push_str("    raise NotImplementedError('Handler not implemented')\n");

    content
}

pub fn generate_events(schema: &Schema, output_dir: &Path) -> Result<()> {
    let events_dir = output_dir.join("generated/events");

    for event in &schema.events {
        let content = generate_event_content(event);
        let file_name = format!("{}.py", templates::to_snake_case(&event.name));
        fs::write(events_dir.join(file_name), content)?;
    }

    let handlers_dir = output_dir.join("handlers/events");
    for event in &schema.events {
        for handler in &event.handlers {
            let file_name = format!("{}.py", handler);
            let handler_path = handlers_dir.join(&file_name);

            if !handler_path.exists() {
                let content = generate_event_handler_stub(event, handler);
                fs::write(handler_path, content)?;
            }
        }
    }

    Ok(())
}

fn generate_event_content(event: &Event) -> String {
    let mut content = String::new();

    content.push_str("from pydantic import BaseModel\n");
    content.push_str("from datetime import datetime\n");
    content.push_str("from typing import Callable, Awaitable\n");

    let payload_field_type = FieldType::from_str(&event.payload);
    let payload_py_type = payload_field_type.to_python();

    let is_custom_type = matches!(payload_field_type, FieldType::Custom(_));
    if is_custom_type {
        content.push_str(&format!(
            "from ..models.{} import {}\n",
            templates::to_snake_case(&event.payload),
            event.payload
        ));
    }

    content.push_str(&format!("\nclass {}(BaseModel):\n", event.name));
    content.push_str(&format!("    payload: {}\n", payload_py_type));
    content.push_str("    timestamp: datetime\n\n");

    content.push_str("    class Config:\n");
    content.push_str("        from_attributes = True\n\n");

    content.push_str(&format!(
        "{}Handler = Callable[[{}], Awaitable[None]]\n",
        event.name, event.name
    ));

    content
}

fn generate_event_handler_stub(event: &Event, handler_name: &str) -> String {
    let mut content = String::new();

    content.push_str(&format!(
        "from generated.events.{} import {}\n\n",
        templates::to_snake_case(&event.name),
        event.name
    ));

    content.push_str(&format!(
        "async def {}(event: {}) -> None:\n",
        handler_name, event.name
    ));
    content.push_str("    # TODO: Implement event handler\n");
    content.push_str(&format!("    print(f'Handling event: {{event}}')\n"));

    content
}

pub fn generate_crons(schema: &Schema, output_dir: &Path) -> Result<()> {
    let handlers_dir = output_dir.join("handlers/cron");

    for cron in &schema.crons {
        let file_name = format!("{}.py", templates::to_snake_case(&cron.name));
        let handler_path = handlers_dir.join(&file_name);

        if !handler_path.exists() {
            let content = format!(
                "async def handle_{}() -> None:\n    # TODO: Implement cron job\n    print('Running cron: {}')\n",
                templates::to_snake_case(&cron.name),
                cron.name
            );
            fs::write(handler_path, content)?;
        }
    }

    Ok(())
}

pub fn generate_websockets(schema: &Schema, output_dir: &Path) -> Result<()> {
    let ws_dir = output_dir.join("generated/websockets");

    for ws in &schema.websockets {
        let content = generate_websocket_content(ws);
        let file_name = format!("{}.py", templates::to_snake_case(&ws.name));
        fs::write(ws_dir.join(file_name), content)?;
    }

    let handlers_dir = output_dir.join("handlers/websockets");
    for ws in &schema.websockets {
        if !ws.on_connect.is_empty() {
            for handler in &ws.on_connect {
                let file_name = format!("{}.py", handler);
                let handler_path = handlers_dir.join(&file_name);
                if !handler_path.exists() {
                    let content = generate_websocket_handler_stub(ws, "onConnect", handler);
                    fs::write(handler_path, content)?;
                }
            }
        }
        if !ws.on_message.is_empty() {
            for handler in &ws.on_message {
                let file_name = format!("{}.py", handler);
                let handler_path = handlers_dir.join(&file_name);
                if !handler_path.exists() {
                    let content = generate_websocket_handler_stub(ws, "onMessage", handler);
                    fs::write(handler_path, content)?;
                }
            }
        }
        if !ws.on_disconnect.is_empty() {
            for handler in &ws.on_disconnect {
                let file_name = format!("{}.py", handler);
                let handler_path = handlers_dir.join(&file_name);
                if !handler_path.exists() {
                    let content = generate_websocket_handler_stub(ws, "onDisconnect", handler);
                    fs::write(handler_path, content)?;
                }
            }
        }
    }

    Ok(())
}

pub fn generate_middlewares(schema: &Schema, output_dir: &Path) -> Result<()> {
    use std::collections::HashSet;
   
    let mut middleware_names = HashSet::new();
    
    for api in &schema.apis {
        for middleware in &api.middlewares {
            middleware_names.insert(middleware.clone());
        }
    }
    
    for ws in &schema.websockets {
        for middleware in &ws.middlewares {
            middleware_names.insert(middleware.clone());
        }
    }
    
    if middleware_names.is_empty() {
        return Ok(());
    }
    
    let middlewares_dir = output_dir.join("middlewares");
    for middleware_name in middleware_names {
        let file_name = format!("{}.py", templates::to_snake_case(&middleware_name));
        let middleware_path = middlewares_dir.join(&file_name);
        
        if !middleware_path.exists() {
            let content = generate_middleware_stub(&middleware_name);
            fs::write(middleware_path, content)?;
        }
    }
    
    Ok(())
}

fn generate_middleware_stub(middleware_name: &str) -> String {
    let mut content = String::new();
    
    content.push_str("from typing import Dict, Any, Optional\n");
    content.push_str("from generated.state import State\n\n");
    
    content.push_str(&format!(
        "async def {}_middleware(context: Dict[str, Any], state: State) -> Optional[Dict[str, Any]]:\n",
        templates::to_snake_case(middleware_name)
    ));
    content.push_str("    \"\"\"\n");
    content.push_str(&format!("    Middleware function for {}.\n\n", middleware_name));
    content.push_str("    Args:\n");
    content.push_str("        context: Request context containing:\n");
    content.push_str("            - payload: Request payload (for APIs)\n");
    content.push_str("            - query_params: Query parameters (for APIs)\n");
    content.push_str("            - connection: WebSocket connection info (for WebSockets)\n");
    content.push_str("            - websocket_name: WebSocket name (for WebSockets)\n");
    content.push_str("            - api_name: API name (for APIs)\n");
    content.push_str("            - trace_id: Trace ID\n");
    content.push_str("        state: State object for logging and triggering events\n\n");
    content.push_str("    Returns:\n");
    content.push_str("        Optional[Dict[str, Any]]: Modified context with 'payload' and/or 'query_params' keys,\n");
    content.push_str("        or None to pass through unchanged. Return a dict with 'error' key to reject the request.\n\n");
    content.push_str("    To reject the request, raise an exception \n");
    content.push_str("    \"\"\"\n");
    content.push_str("    # TODO: Implement middleware logic\n");
    content.push_str("    # Example: Validate authentication\n");
    content.push_str("    # Example: Rate limiting\n");
    content.push_str("    # Example: Logging\n");
    content.push_str("    # Example: Modify payload/query_params\n");
    content.push_str("    # \n");
    content.push_str("    # To modify the request:\n");
    content.push_str("    # return {\n");
    content.push_str("    #     'payload': modified_payload,\n");
    content.push_str("    #     'query_params': modified_query_params\n");
    content.push_str("    # }\n");
    content.push_str("    # \n");
    content.push_str("    # To reject the request:\n");
    content.push_str("    # raise Exception('Access denied')\n");
    content.push_str("    \n");
    content.push_str("    # Pass through unchanged\n");
    content.push_str("    return None\n");
    
    content
}

fn generate_websocket_content(ws: &WebSocket) -> String {
    let mut content = String::new();

    content.push_str("from pydantic import BaseModel\n");
    content.push_str("from typing import Dict, Any, Optional\n");
    content.push_str("from datetime import datetime\n\n");

    if let Some(message_type) = &ws.message {
        let message_field_type = FieldType::from_str(message_type);
        let is_custom_type = matches!(message_field_type, FieldType::Custom(_));
        if is_custom_type {
            content.push_str(&format!(
                "from ..dto.{} import {}\n",
                templates::to_snake_case(message_type),
                message_type
            ));
        }
    }

    content.push_str(&format!("class {}Message(BaseModel):\n", ws.name));
    if let Some(message_type) = &ws.message {
        let message_field_type = FieldType::from_str(message_type);
        let py_type = message_field_type.to_python();
        content.push_str(&format!("    data: {}\n", py_type));
    } else {
        content.push_str("    data: Dict[str, Any]\n");
    }
    content.push_str("    timestamp: datetime\n\n");
    content.push_str("    class Config:\n");
    content.push_str("        from_attributes = True\n\n");

    content.push_str(&format!("class {}Connection(BaseModel):\n", ws.name));
    content.push_str("    connection_id: str\n");
    content.push_str("    path: str\n");
    content.push_str("    connected_at: datetime\n\n");
    content.push_str("    class Config:\n");
    content.push_str("        from_attributes = True\n");

    content
}

fn generate_websocket_handler_stub(
    ws: &WebSocket,
    handler_type: &str,
    handler_name: &str,
) -> String {
    let mut content = String::new();

    content.push_str(&format!(
        "from generated.websockets.{} import {}Message, {}Connection\n",
        templates::to_snake_case(&ws.name),
        ws.name,
        ws.name
    ));
    content.push_str("from generated.state import State\n");
    content.push_str("from typing import Optional\n\n");

    match handler_type {
        "onConnect" => {
            content.push_str(&format!(
                "async def {}(connection: {}Connection, state: State) -> Optional[{}Message]:\n",
                handler_name, ws.name, ws.name
            ));
            content.push_str("    # TODO: Implement onConnect handler\n");
            content
                .push_str("    # Return a message to send to the client on connection, or None\n");
            content.push_str(&format!(
                "    print(f'Client connected: {{connection.connection_id}}')\n"
            ));
            content.push_str("    return None\n");
        }
        "onMessage" => {
            content.push_str(&format!(
                "async def {}(message: {}Message, connection: {}Connection, state: State) -> Optional[{}Message]:\n",
                handler_name,
                ws.name,
                ws.name,
                ws.name
            ));
            content.push_str("    # TODO: Implement onMessage handler\n");
            content.push_str("    # Return a message to send back to the client, or None\n");
            content.push_str(&format!(
                "    print(f'Received message: {{message.data}}')\n"
            ));
            content.push_str("    # For auto-triggers (defined in schema triggers): use state.set_payload('EventName', {...})\n");
            content.push_str(
                "    # For manual triggers: use state.trigger_event('EventName', {...})\n",
            );
            content.push_str("    return None\n");
        }
        "onDisconnect" => {
            content.push_str(&format!(
                "async def {}(connection: {}Connection, state: State) -> None:\n",
                handler_name, ws.name
            ));
            content.push_str("    # TODO: Implement onDisconnect handler\n");
            content.push_str(&format!(
                "    print(f'Client disconnected: {{connection.connection_id}}')\n"
            ));
        }
        _ => {}
    }

    content
}

pub fn generate_state(output_dir: &Path) -> Result<()> {
    let generated_dir = output_dir.join("generated");
    let content = r#"from typing import Any, Dict, List, Optional
from pydantic import BaseModel


class TriggeredEvent(BaseModel):
    event_name: str
    payload: Dict[str, Any]


class Logger:
    """Logger for handlers to emit structured logs."""
    
    def __init__(self, handler_name: str, log_fn: Any):
        self._handler_name = handler_name
        self._log_fn = log_fn
    
    def info(self, message: str, **kwargs: Any) -> None:
        """Log an info message.
        
        Args:
            message: Log message
            **kwargs: Additional fields to include in the log
        """
        if self._log_fn:
            self._log_fn("info", self._handler_name, message, kwargs)
    
    def error(self, message: str, **kwargs: Any) -> None:
        """Log an error message.
        
        Args:
            message: Log message
            **kwargs: Additional fields to include in the log
        """
        if self._log_fn:
            self._log_fn("error", self._handler_name, message, kwargs)
    
    def warning(self, message: str, **kwargs: Any) -> None:
        """Log a warning message.
        
        Args:
            message: Log message
            **kwargs: Additional fields to include in the log
        """
        if self._log_fn:
            self._log_fn("warn", self._handler_name, message, kwargs)
    
    def warn(self, message: str, **kwargs: Any) -> None:
        """Log a warning message (alias for warning).
        
        Args:
            message: Log message
            **kwargs: Additional fields to include in the log
        """
        self.warning(message, **kwargs)
    
    def debug(self, message: str, **kwargs: Any) -> None:
        """Log a debug message.
        
        Args:
            message: Log message
            **kwargs: Additional fields to include in the log
        """
        if self._log_fn:
            self._log_fn("debug", self._handler_name, message, kwargs)
    
    def trace(self, message: str, **kwargs: Any) -> None:
        """Log a trace message.
        
        Args:
            message: Log message
            **kwargs: Additional fields to include in the log
        """
        if self._log_fn:
            self._log_fn("trace", self._handler_name, message, kwargs)


class State:
    """Context object for handlers to trigger events and access runtime state."""
    
    def __init__(self, handler_name: Optional[str] = None, log_fn: Optional[Any] = None):
        self._triggers: List[TriggeredEvent] = []
        self._auto_trigger_payloads: Dict[str, Dict[str, Any]] = {}
        self.logger = Logger(handler_name or "unknown", log_fn)
    
    def trigger_event(self, event_name: str, payload: Dict[str, Any]) -> None:
        """Manually trigger an event with the given payload.
        
        Use this for events that are NOT defined in the schema's triggers list.
        
        Args:
            event_name: Name of the event to trigger
            payload: Event payload data (will be serialized to JSON)
        """
        self._triggers.append(TriggeredEvent(
            event_name=event_name,
            payload=payload
        ))
    
    def set_payload(self, event_name: str, payload: Dict[str, Any]) -> None:
        """Set the payload for an auto-triggered event.
        
        Use this for events that ARE defined in the schema's triggers list.
        The event will be automatically triggered after the handler completes,
        using the payload you set here.
        
        Args:
            event_name: Name of the event (must match a trigger in schema)
            payload: Event payload data (will be serialized to JSON)
        """
        self._auto_trigger_payloads[event_name] = payload
    
    def get_triggers(self) -> List[TriggeredEvent]:
        """Get all manually triggered events. Used internally by the runtime."""
        return self._triggers.copy()
    
    def get_auto_trigger_payload(self, event_name: str) -> Optional[Dict[str, Any]]:
        """Get payload for an auto-triggered event. Used internally by the runtime."""
        return self._auto_trigger_payloads.get(event_name)
    
    def get_all_auto_trigger_payloads(self) -> Dict[str, Dict[str, Any]]:
        """Get all auto-trigger payloads. Used internally by the runtime."""
        return self._auto_trigger_payloads.copy()
"#;

    fs::write(generated_dir.join("state.py"), content)?;
    Ok(())
}

pub fn generate_init(schema: &Schema, output_dir: &Path) -> Result<()> {
    let generated_dir = output_dir.join("generated");

    let subdirs = ["models", "dto", "api", "events", "cron", "websockets"];
    for subdir in &subdirs {
        fs::write(generated_dir.join(format!("{}/__init__.py", subdir)), "")?;
    }

    let mut content = String::new();
    content.push_str("# Generated by Rohas - Do not edit\n\n");

    content.push_str("from .state import State, TriggeredEvent\n");

    for model in &schema.models {
        content.push_str(&format!(
            "from .models.{} import {}\n",
            templates::to_snake_case(&model.name),
            model.name
        ));
    }

    fs::write(generated_dir.join("__init__.py"), content)?;

    Ok(())
}
