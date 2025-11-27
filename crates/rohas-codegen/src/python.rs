use crate::error::Result;
use crate::templates;
use rohas_parser::{Api, Event, FieldType, Model, Schema};
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

pub fn generate_state(output_dir: &Path) -> Result<()> {
    let generated_dir = output_dir.join("generated");
    let content = r#"from typing import Any, Dict, List, Optional
from pydantic import BaseModel


class TriggeredEvent(BaseModel):
    event_name: str
    payload: Dict[str, Any]


class State:
    """Context object for handlers to trigger events and access runtime state."""
    
    def __init__(self):
        self._triggers: List[TriggeredEvent] = []
        self._auto_trigger_payloads: Dict[str, Dict[str, Any]] = {}
    
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

    let subdirs = ["models", "dto", "api", "events", "cron"];
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
