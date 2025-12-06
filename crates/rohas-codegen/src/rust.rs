use crate::error::Result;
use crate::templates;
use rohas_parser::{Api, Event, FieldType, Model, Schema, WebSocket};
use std::fs;
use std::path::Path;

/// Rust reserved keywords that need to be escaped with r#
const RUST_RESERVED_KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
    "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move",
    "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true",
    "type", "unsafe", "use", "where", "while",
];

fn escape_rust_keyword(name: &str) -> String {
    if RUST_RESERVED_KEYWORDS.contains(&name) {
        format!("r#{}", name)
    } else {
        name.to_string()
    }
}

pub fn generate_models(schema: &Schema, output_dir: &Path) -> Result<()> {
    let models_dir = output_dir.join("generated/models");

    for model in &schema.models {
        let content = generate_model_content(model);
        let file_name = format!("{}.rs", templates::to_snake_case(&model.name));
        fs::write(models_dir.join(file_name), content)?;
    }

    let mut mod_content = String::new();
    mod_content.push_str("// Auto-generated module declarations\n");
    for model in &schema.models {
        let mod_name = templates::to_snake_case(&model.name);
        mod_content.push_str(&format!("pub mod {};\n", mod_name));
        mod_content.push_str(&format!("pub use {}::{};\n", mod_name, model.name));
    }
    fs::write(models_dir.join("mod.rs"), mod_content)?;

    Ok(())
}

fn generate_model_content(model: &Model) -> String {
    let mut content = String::new();

    content.push_str("use serde::{Deserialize, Serialize};\n\n");
    content.push_str(&format!("#[derive(Debug, Clone, Serialize, Deserialize)]\n"));
    content.push_str(&format!("pub struct {}\n", model.name));
    content.push_str("{\n");

    for field in &model.fields {
        let rust_type = field.field_type.to_rust();
        let type_hint = if field.optional {
            format!("Option<{}>", rust_type)
        } else {
            rust_type
        };

        let field_name = escape_rust_keyword(&field.name);
        let serde_attr = if RUST_RESERVED_KEYWORDS.contains(&field.name.as_str()) {
            format!("    #[serde(rename = \"{}\")]\n", field.name)
        } else {
            String::new()
        };
        content.push_str(&serde_attr);
        content.push_str(&format!("    pub {}: {},\n", field_name, type_hint));
    }

    if model.fields.is_empty() {
        content.push_str("    // No fields\n");
    }

    content.push_str("}\n");

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
        let file_name = format!("{}.rs", templates::to_snake_case(&input.name));
        fs::write(dto_dir.join(file_name), content)?;
    }

    let mut mod_content = String::new();
    mod_content.push_str("// Auto-generated module declarations\n");
    for input in &schema.inputs {
        let mod_name = templates::to_snake_case(&input.name);
        mod_content.push_str(&format!("pub mod {};\n", mod_name));
        mod_content.push_str(&format!("pub use {}::{};\n", mod_name, input.name));
    }
    fs::write(dto_dir.join("mod.rs"), mod_content)?;

    Ok(())
}

pub fn generate_apis(schema: &Schema, output_dir: &Path) -> Result<()> {
    let api_dir = output_dir.join("generated/api");

    for api in &schema.apis {
        let content = generate_api_content(api);
        let file_name = format!("{}.rs", templates::to_snake_case(&api.name));
        fs::write(api_dir.join(file_name), content)?;
    }

    let mut mod_content = String::new();
    mod_content.push_str("// Auto-generated module declarations\n");
    for api in &schema.apis {
        let mod_name = templates::to_snake_case(&api.name);
        mod_content.push_str(&format!("pub mod {};\n", mod_name));
        mod_content.push_str(&format!("pub use {}::{{ {}Request, {}Response }};\n", mod_name, api.name, api.name));
    }
    fs::write(api_dir.join("mod.rs"), mod_content)?;

    let handlers_dir = output_dir.join("handlers/api");
    for api in &schema.apis {
        let file_name = format!("{}.rs", templates::to_snake_case(&api.name));
        let handler_path = handlers_dir.join(&file_name);

        if !handler_path.exists() {
            let content = generate_api_handler_stub(api);
            fs::write(handler_path, content)?;
        }
    }

    Ok(())
}

fn generate_api_content(api: &Api) -> String {
    let mut content = String::new();

    content.push_str("use serde::{Deserialize, Serialize};\n");

    if let Some(body_type) = &api.body {
        let body_type_snake = templates::to_snake_case(body_type);
        if body_type.ends_with("Input") {
            content.push_str(&format!("use crate::generated::dto::{}::{};\n", body_type_snake, body_type));
        } else {
            content.push_str(&format!("use crate::generated::models::{}::{};\n", body_type_snake, body_type));
        }
    }

    let response_field_type = rohas_parser::FieldType::from_str(&api.response);
    let is_custom_response = matches!(response_field_type, rohas_parser::FieldType::Custom(_));
    if is_custom_response {
        let response_type_snake = templates::to_snake_case(&api.response);
        content.push_str(&format!("use crate::generated::models::{}::{};\n", response_type_snake, api.response));
    }
    content.push_str("\n");

    if let Some(body_type) = &api.body {
        content.push_str(&format!(
            "pub type {}Request = {};\n\n",
            api.name, body_type
        ));
    } else {
        content.push_str(&format!(
            "#[derive(Debug, Clone, Serialize, Deserialize)]\n"
        ));
        content.push_str(&format!("pub struct {}Request\n", api.name));
        content.push_str("{\n");
        content.push_str("    // No body fields\n");
        content.push_str("}\n\n");
    }

    let response_rust_type = response_field_type.to_rust();
    content.push_str(&format!(
        "pub type {}Response = {};\n",
        api.name, response_rust_type
    ));

    content
}

fn generate_api_handler_stub(api: &Api) -> String {
    let mut content = String::new();

    let request_type = format!("{}Request", api.name);
    let response_type = format!("{}Response", api.name);
    let handler_name = format!("handle_{}", templates::to_snake_case(&api.name));
    let module_name = templates::to_snake_case(&api.name);

    content.push_str(&format!(
        "use crate::generated::api::{}::{{ {}, {} }};\n",
        module_name, request_type, response_type
    ));
    content.push_str("use crate::generated::state::State;\n");
    content.push_str("use rohas_runtime::{HandlerContext, HandlerResult, Result};\n\n");

    content.push_str(&format!(
        "/// Rust handler for {} API.\n",
        api.name
    ));
    content.push_str(&format!(
        "pub async fn {}(\n",
        handler_name
    ));
    content.push_str(&format!("    req: {},\n", request_type));
    content.push_str("    state: &mut State,\n");
    content.push_str(&format!(") -> Result<{}> {{\n", response_type));
    content.push_str("    // TODO: Implement handler logic\n");
    content.push_str("    // For auto-triggers (defined in schema triggers): use state.set_payload(\"EventName\", value)\n");
    content.push_str("    // For manual triggers: use state.trigger_event(\"EventName\", value)\n");
    content.push_str("    // Use state.logger for structured logging\n");
    content.push_str(&format!(
        "    Err(rohas_runtime::RuntimeError::ExecutionFailed(\"Handler not implemented\".into()))\n"
    ));
    content.push_str("}\n");

    content
}

pub fn generate_events(schema: &Schema, output_dir: &Path) -> Result<()> {
    let events_dir = output_dir.join("generated/events");

    for event in &schema.events {
        let content = generate_event_content(event);
        let file_name = format!("{}.rs", templates::to_snake_case(&event.name));
        fs::write(events_dir.join(file_name), content)?;
    }

    let mut mod_content = String::new();
    mod_content.push_str("// Auto-generated module declarations\n");
    for event in &schema.events {
        let mod_name = templates::to_snake_case(&event.name);
        mod_content.push_str(&format!("pub mod {};\n", mod_name));
        mod_content.push_str(&format!("pub use {}::{};\n", mod_name, event.name));
    }
    fs::write(events_dir.join("mod.rs"), mod_content)?;

    let handlers_dir = output_dir.join("handlers/events");
    for event in &schema.events {
        for handler in &event.handlers {
            let file_name = format!("{}.rs", handler);
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

    content.push_str("use serde::{Deserialize, Serialize};\n");
    content.push_str("use chrono::{DateTime, Utc};\n\n");

    let payload_field_type = FieldType::from_str(&event.payload);
    let payload_rust_type = payload_field_type.to_rust();

    let is_custom_type = matches!(payload_field_type, FieldType::Custom(_));
    if is_custom_type {
        let model_module = templates::to_snake_case(&event.payload);
        content.push_str(&format!(
            "use crate::generated::models::{}::{};\n",
            model_module, event.payload
        ));
    }

    content.push_str(&format!("#[derive(Debug, Clone, Serialize, Deserialize)]\n"));
    content.push_str(&format!("pub struct {}\n", event.name));
    content.push_str("{\n");
    content.push_str(&format!("    pub payload: {},\n", payload_rust_type));
    content.push_str("    pub timestamp: DateTime<Utc>,\n");
    content.push_str("}\n");

    content
}

fn generate_event_handler_stub(event: &Event, handler_name: &str) -> String {
    let mut content = String::new();

    let event_module = templates::to_snake_case(&event.name);

    content.push_str(&format!(
        "use crate::generated::events::{}::{};\n",
        event_module, event.name
    ));
    content.push_str("use rohas_runtime::{HandlerContext, HandlerResult, Result};\n\n");

    content.push_str(&format!(
        "/// High-performance Rust event handler.\n"
    ));
    content.push_str(&format!(
        "pub async fn {}(\n",
        handler_name
    ));
    content.push_str(&format!("    event: {},\n", event.name));
    content.push_str(") -> Result<HandlerResult> {\n");
    content.push_str("    // TODO: Implement event handler\n");
    content.push_str(&format!(
        "    tracing::info!(\"Handling event: {{:?}}\", event);\n"
    ));
    content.push_str("    Ok(HandlerResult::success(serde_json::json!({}), 0))\n");
    content.push_str("}\n");

    content
}

pub fn generate_crons(schema: &Schema, output_dir: &Path) -> Result<()> {
    let handlers_dir = output_dir.join("handlers/cron");

    for cron in &schema.crons {
        let file_name = format!("{}.rs", templates::to_snake_case(&cron.name));
        let handler_path = handlers_dir.join(&file_name);

        if !handler_path.exists() {
            let content = generate_cron_handler_stub(cron);
            fs::write(handler_path, content)?;
        }
    }

    Ok(())
}

fn generate_cron_handler_stub(cron: &rohas_parser::Cron) -> String {
    let mut content = String::new();

    let handler_name = format!("handle_{}", templates::to_snake_case(&cron.name));

    content.push_str("use rohas_runtime::{HandlerContext, HandlerResult, Result};\n");
    content.push_str("use crate::generated::state::State;\n\n");

    content.push_str(&format!(
        "/// High-performance Rust cron handler.\n"
    ));
    content.push_str(&format!(
        "pub async fn {}(\n",
        handler_name
    ));
    content.push_str("    state: &mut State,\n");
    content.push_str(") -> Result<HandlerResult> {\n");
    content.push_str("    // TODO: Implement cron handler\n");
    content.push_str(&format!(
        "    tracing::info!(\"Executing cron: {}\");\n",
        cron.name
    ));
    content.push_str("    Ok(HandlerResult::success(serde_json::json!({}), 0))\n");
    content.push_str("}\n");

    content
}

pub fn generate_websockets(schema: &Schema, output_dir: &Path) -> Result<()> {
    let ws_dir = output_dir.join("generated/websockets");
    
    fs::create_dir_all(&ws_dir)?;

    for ws in &schema.websockets {
        let content = generate_websocket_content(ws, schema);
        let file_name = format!("{}.rs", templates::to_snake_case(&ws.name));
        let file_path = ws_dir.join(&file_name);
        fs::write(&file_path, content).map_err(|e| {
            crate::error::CodegenError::Io(std::io::Error::new(
                e.kind(),
                format!(
                    "Failed to write websocket file {}: {}",
                    file_path.display(),
                    e
                )
            ))
        })?;
    }

    let mut mod_content = String::new();
    mod_content.push_str("// Auto-generated module declarations\n");
    for ws in &schema.websockets {
        let mod_name = templates::to_snake_case(&ws.name);
        mod_content.push_str(&format!("pub mod {};\n", mod_name));
        mod_content.push_str(&format!("pub use {}::{{ {}Connection", mod_name, ws.name));
        if ws.message.is_some() {
            mod_content.push_str(&format!(", {}Message", ws.name));
        }
        mod_content.push_str(" };\n");
    }
    fs::write(ws_dir.join("mod.rs"), mod_content)?;

    let handlers_dir = output_dir.join("handlers/websockets");
    fs::create_dir_all(&handlers_dir)?;
    
    for ws in &schema.websockets {
        for handler in &ws.on_connect {
            let file_name = format!("{}.rs", handler);
            let handler_path = handlers_dir.join(&file_name);
            if !handler_path.exists() {
                let content = generate_websocket_handler_stub(ws, handler, "connect");
                fs::write(&handler_path, content).map_err(|e| {
                    crate::error::CodegenError::Io(std::io::Error::new(
                        e.kind(),
                        format!(
                            "Failed to write websocket handler {}: {}",
                            handler_path.display(),
                            e
                        )
                    ))
                })?;
            }
        }
        for handler in &ws.on_message {
            let file_name = format!("{}.rs", handler);
            let handler_path = handlers_dir.join(&file_name);
            if !handler_path.exists() {
                let content = generate_websocket_handler_stub(ws, handler, "message");
                fs::write(&handler_path, content).map_err(|e| {
                    crate::error::CodegenError::Io(std::io::Error::new(
                        e.kind(),
                        format!(
                            "Failed to write websocket handler {}: {}",
                            handler_path.display(),
                            e
                        )
                    ))
                })?;
            }
        }
        for handler in &ws.on_disconnect {
            let file_name = format!("{}.rs", handler);
            let handler_path = handlers_dir.join(&file_name);
            if !handler_path.exists() {
                let content = generate_websocket_handler_stub(ws, handler, "disconnect");
                fs::write(&handler_path, content).map_err(|e| {
                    crate::error::CodegenError::Io(std::io::Error::new(
                        e.kind(),
                        format!(
                            "Failed to write websocket handler {}: {}",
                            handler_path.display(),
                            e
                        )
                    ))
                })?;
            }
        }
    }

    Ok(())
}

fn generate_websocket_content(ws: &WebSocket, schema: &Schema) -> String {
    let mut content = String::new();

    content.push_str("use serde::{Deserialize, Serialize};\n");
    if ws.message.is_some() {
        content.push_str("use chrono::{DateTime, Utc};\n");
    }
    content.push_str("\n");

    if let Some(message_type) = &ws.message {
        let message_field_type = FieldType::from_str(message_type);
        let is_custom_type = matches!(message_field_type, FieldType::Custom(_));
        
        let rust_type = message_field_type.to_rust();
        
        if is_custom_type {
            let message_type_snake = templates::to_snake_case(message_type);
            // Check if it's an input/DTO type
            let is_input = schema.inputs.iter().any(|input| input.name == *message_type) 
                || message_type.ends_with("Input");
            
            if is_input {
                content.push_str(&format!(
                    "use crate::generated::dto::{}::{};\n",
                    message_type_snake, message_type
                ));
            } else {
                content.push_str(&format!(
                    "use crate::generated::models::{}::{};\n",
                    message_type_snake, message_type
                ));
            }
        }
        

        content.push_str(&format!(
            "#[derive(Debug, Clone, Serialize, Deserialize)]\n"
        ));
        content.push_str(&format!("pub struct {}Message\n", ws.name));
        content.push_str("{\n");
        content.push_str(&format!("    pub data: {},\n", rust_type));
        content.push_str("    pub timestamp: chrono::DateTime<chrono::Utc>,\n");
        content.push_str("}\n\n");
    }

    content.push_str(&format!(
        "#[derive(Debug, Clone, Serialize, Deserialize)]\n"
    ));
    content.push_str(&format!("pub struct {}Connection\n", ws.name));
    content.push_str("{\n");
    content.push_str("    // Connection metadata\n");
    content.push_str("}\n");

    content
}

fn generate_websocket_handler_stub(ws: &WebSocket, handler_name: &str, event_type: &str) -> String {
    let mut content = String::new();

    let ws_module = templates::to_snake_case(&ws.name);

    content.push_str(&format!(
        "use crate::generated::websockets::{}::{}Connection;\n",
        ws_module, ws.name
    ));

    if ws.message.is_some() {
        content.push_str(&format!(
            "use crate::generated::websockets::{}::{}Message;\n",
            ws_module, ws.name
        ));
    }

    content.push_str("use rohas_runtime::{HandlerContext, HandlerResult, Result};\n");
    content.push_str("use crate::generated::state::State;\n\n");

    content.push_str(&format!(
        "/// Rust WebSocket {} handler.\n",
        event_type
    ));
    content.push_str(&format!("pub async fn {}(\n", handler_name));

    if event_type == "message" {
        if let Some(_) = &ws.message {
            content.push_str(&format!("    message: {}Message,\n", ws.name));
        }
        content.push_str(&format!("    connection: {}Connection,\n", ws.name));
        content.push_str("    state: &mut State,\n");
    } else {
        content.push_str(&format!("    connection: {}Connection,\n", ws.name));
        if event_type == "connect" {
            content.push_str("    state: &mut State,\n");
        }
    }

    content.push_str(") -> Result<HandlerResult> {\n");
    content.push_str(&format!(
        "    tracing::info!(\"WebSocket {} handler: {{:?}}\", connection);\n",
        event_type
    ));
    content.push_str("    Ok(HandlerResult::success(serde_json::json!({}), 0))\n");
    content.push_str("}\n");

    content
}

pub fn generate_middlewares(schema: &Schema, output_dir: &Path) -> Result<()> {
    let mut middleware_names = std::collections::HashSet::new();

    for api in &schema.apis {
        for mw in &api.middlewares {
            middleware_names.insert(mw.clone());
        }
    }

    for ws in &schema.websockets {
        for mw in &ws.middlewares {
            middleware_names.insert(mw.clone());
        }
    }

    let middlewares_dir = output_dir.join("middlewares");
    fs::create_dir_all(&middlewares_dir)?;
    
    for mw_name in middleware_names {
        let file_name = format!("{}.rs", templates::to_snake_case(&mw_name));
        let handler_path = middlewares_dir.join(&file_name);

        if !handler_path.exists() {
            let content = generate_middleware_stub(&mw_name);
            fs::write(&handler_path, content).map_err(|e| {
                crate::error::CodegenError::Io(std::io::Error::new(
                    e.kind(),
                    format!(
                        "Failed to write middleware handler {}: {}",
                        handler_path.display(),
                        e
                    )
                ))
            })?;
        }
    }

    Ok(())
}

fn generate_middleware_stub(mw_name: &str) -> String {
    let mut content = String::new();

    let handler_name = format!("{}_middleware", templates::to_snake_case(mw_name));

    content.push_str("use rohas_runtime::{HandlerContext, HandlerResult, Result};\n");
    content.push_str("use crate::generated::state::State;\n\n");

    content.push_str(&format!(
        "/// High-performance Rust middleware.\n"
    ));
    content.push_str(&format!("pub async fn {}(\n", handler_name));
    content.push_str("    ctx: HandlerContext,\n");
    content.push_str("    state: &mut State,\n");
    content.push_str(") -> Result<HandlerResult> {\n");
    content.push_str("    // TODO: Implement middleware logic\n");
    content.push_str("    // Return Ok to continue, Err to abort\n");
    content.push_str(&format!(
        "    tracing::info!(\"Middleware {} executed\");\n",
        mw_name
    ));
    content.push_str("    Ok(HandlerResult::success(serde_json::json!({}), 0))\n");
    content.push_str("}\n");

    content
}

pub fn generate_state(output_dir: &Path) -> Result<()> {
    let generated_dir = output_dir.join("generated");
    let content = r#"use serde_json::Value;
use std::collections::HashMap;
use tracing::{error, warn, info, debug, trace};

/// State struct for Rust handlers.
#[derive(Debug, Clone)]
pub struct State {
    handler_name: String,
    triggers: Vec<TriggeredEvent>,
    auto_trigger_payloads: HashMap<String, Value>,
}

#[derive(Debug, Clone)]
pub struct TriggeredEvent {
    pub event_name: String,
    pub payload: Value,
}

impl State {
    /// Create a new State instance.
    pub fn new(handler_name: impl Into<String>) -> Self {
        Self {
            handler_name: handler_name.into(),
            triggers: Vec::new(),
            auto_trigger_payloads: HashMap::new(),
        }
    }

    /// Manually trigger an event (for events NOT in schema triggers).
    pub fn trigger_event(&mut self, event_name: impl Into<String>, payload: Value) {
        self.triggers.push(TriggeredEvent {
            event_name: event_name.into(),
            payload,
        });
    }

    /// Set payload for an auto-triggered event (for events IN schema triggers).
    pub fn set_payload(&mut self, event_name: impl Into<String>, payload: Value) {
        self.auto_trigger_payloads.insert(event_name.into(), payload);
    }

    /// Get all manually triggered events (internal use).
    pub fn get_triggers(&self) -> &[TriggeredEvent] {
        &self.triggers
    }

    /// Get all auto-trigger payloads (internal use).
    pub fn get_all_auto_trigger_payloads(&self) -> &HashMap<String, Value> {
        &self.auto_trigger_payloads
    }

    /// Get a logger instance for this handler.
    pub fn logger(&self) -> Logger {
        Logger::new(&self.handler_name)
    }
}

/// Structured logger for handlers.
pub struct Logger {
    handler_name: String,
}

impl Logger {
    pub fn new(handler_name: impl Into<String>) -> Self {
        Self {
            handler_name: handler_name.into(),
        }
    }

    pub fn info(&self, message: &str) {
        info!(handler = %self.handler_name, %message);
    }

    pub fn error(&self, message: &str) {
        error!(handler = %self.handler_name, %message);
    }

    pub fn warn(&self, message: &str) {
        warn!(handler = %self.handler_name, %message);
    }

    pub fn debug(&self, message: &str) {
        debug!(handler = %self.handler_name, %message);
    }

    pub fn trace(&self, message: &str) {
        trace!(handler = %self.handler_name, %message);
    }
}
"#;

    fs::write(generated_dir.join("state.rs"), content)?;
    Ok(())
}

/// Generate lib.rs for the generated crate.
pub fn generate_lib_rs(schema: &Schema, output_dir: &Path) -> Result<()> {
    let generated_dir = output_dir.join("generated");

    let mut content = String::new();
    content.push_str("// Auto-generated Rust code from Rohas schema\n");
    content.push_str("// DO NOT EDIT MANUALLY\n\n");

    // Generate module declarations
    content.push_str("pub mod state;\n");
    content.push_str("pub mod models;\n");
    content.push_str("pub mod dto;\n");
    content.push_str("pub mod api;\n");
    content.push_str("pub mod events;\n");
    content.push_str("pub mod websockets;\n");
    content.push_str("pub mod handlers;\n\n");

    // Re-export commonly used types
    content.push_str("pub use state::State;\n");
    content.push_str("pub use handlers::register_all_handlers;\n");
    content.push_str("pub use handlers::set_runtime;\n\n");

    fs::write(generated_dir.join("lib.rs"), content)?;

    // Generate handlers registration module
    generate_handlers_registration(schema, output_dir)?;


    // Also generate the main src/lib.rs that sets up the module structure
    let mut main_lib_content = String::new();
    main_lib_content.push_str("// Main library entry point for Rohas Rust application\n");
    main_lib_content.push_str("// This file sets up the module structure\n\n");
    main_lib_content.push_str("#[path = \"generated/lib.rs\"]\n");
    main_lib_content.push_str("pub mod generated;\n\n");
    main_lib_content.push_str("// Re-export generated types for convenience\n");
    main_lib_content.push_str("pub use generated::*;\n\n");

    // Generate handlers module declarations
    let handlers_dir = output_dir.join("handlers");
    let middlewares_dir = output_dir.join("middlewares");
    if handlers_dir.join("api").exists() || handlers_dir.join("events").exists() || middlewares_dir.exists() {
        main_lib_content.push_str("pub mod handlers;\n\n");
    }
    
    if middlewares_dir.exists() {
        main_lib_content.push_str("pub mod middlewares;\n\n");
    }

    // Add initialization function that can be called to register handlers
    main_lib_content.push_str("/// Initialize and register all handlers with the Rust runtime.\n");
    main_lib_content.push_str("/// This function should be called during engine startup.\n");
    main_lib_content.push_str("/// It will automatically register all handlers using the global registry.\n");
    main_lib_content.push_str("pub async fn init_handlers(runtime: std::sync::Arc<rohas_runtime::RustRuntime>) -> rohas_runtime::Result<()> {\n");
    main_lib_content.push_str("    generated::register_all_handlers(runtime).await\n");
    main_lib_content.push_str("}\n\n");

    // Add a C-compatible FFI function that can be called from the engine
    // This allows the engine to automatically register handlers
    main_lib_content.push_str("/// C-compatible FFI function for automatic handler registration.\n");
    main_lib_content.push_str("/// This is called automatically by the engine.\n");
    main_lib_content.push_str("/// Returns 0 on success, non-zero on error.\n");
    main_lib_content.push_str("#[no_mangle]\n");
    main_lib_content.push_str("pub extern \"C\" fn rohas_set_runtime(runtime_ptr: *mut std::ffi::c_void) -> i32 {\n");
    main_lib_content.push_str("    use std::sync::Arc;\n");
    main_lib_content.push_str("    \n");
    main_lib_content.push_str("    if runtime_ptr.is_null() {\n");
    main_lib_content.push_str("        return 1; // Error: null pointer\n");
    main_lib_content.push_str("    }\n");
    main_lib_content.push_str("    \n");
    main_lib_content.push_str("    // Safety: The engine passes a valid Arc<RustRuntime> pointer that was created with Arc::into_raw.\n");
    main_lib_content.push_str("    // We reconstruct the Arc temporarily to clone it, then forget it so the engine retains ownership.\n");
    main_lib_content.push_str("    unsafe {\n");
    main_lib_content.push_str("        // Convert the raw pointer back to Arc<RustRuntime>\n");
    main_lib_content.push_str("        // The engine created this with Arc::into_raw, so we reconstruct it temporarily\n");
    main_lib_content.push_str("        let runtime: Arc<rohas_runtime::RustRuntime> = Arc::from_raw(runtime_ptr as *const rohas_runtime::RustRuntime);\n");
    main_lib_content.push_str("        \n");
    main_lib_content.push_str("        // Clone the Arc - this increments the reference count\n");
    main_lib_content.push_str("        let runtime_clone = runtime.clone();\n");
    main_lib_content.push_str("        \n");
    main_lib_content.push_str("        // Forget the reconstructed Arc - we don't want to drop it here since the engine still owns it\n");
    main_lib_content.push_str("        // The engine will manage the original Arc's lifetime\n");
    main_lib_content.push_str("        std::mem::forget(runtime);\n");
    main_lib_content.push_str("        \n");
    main_lib_content.push_str("        // Call the generated set_runtime function which will register all handlers\n");
    main_lib_content.push_str("        // This will store the cloned Arc in a OnceLock and register handlers synchronously\n");
    main_lib_content.push_str("        // Note: If registration fails, set_runtime will panic (via .expect())\n");
    main_lib_content.push_str("        generated::set_runtime(runtime_clone);\n");
    main_lib_content.push_str("        \n");
    main_lib_content.push_str("        0 // Success\n");
    main_lib_content.push_str("    }\n");
    main_lib_content.push_str("}\n");

    fs::write(output_dir.join("lib.rs"), main_lib_content)?;

    // Generate handlers/mod.rs if handlers exist
    if handlers_dir.join("api").exists() || handlers_dir.join("events").exists() {
        generate_handlers_mod(schema, output_dir)?;
    }

    Ok(())
}

fn generate_handlers_mod(schema: &Schema, output_dir: &Path) -> Result<()> {
    let handlers_dir = output_dir.join("handlers");
    let middlewares_dir = output_dir.join("middlewares");
    let mut content = String::new();

    content.push_str("// Handler module declarations\n\n");

    if handlers_dir.join("api").exists() {
        content.push_str("pub mod api;\n");
    }

    if handlers_dir.join("events").exists() {
        content.push_str("pub mod events;\n");
    }

    if handlers_dir.join("websockets").exists() {
        content.push_str("pub mod websockets;\n");
    }

    fs::write(handlers_dir.join("mod.rs"), content)?;

    if handlers_dir.join("api").exists() {
        let mut api_mod = String::new();
        api_mod.push_str("// API handler modules\n\n");

        for api in &schema.apis {
            let handler_name = templates::to_snake_case(&api.name);
            let handler_file = handlers_dir.join("api").join(format!("{}.rs", handler_name));
            if handler_file.exists() {
                api_mod.push_str(&format!("pub mod {};\n", handler_name));
            }
        }

        fs::write(handlers_dir.join("api").join("mod.rs"), api_mod)?;
    }

    if handlers_dir.join("events").exists() {
        let mut events_mod = String::new();
        events_mod.push_str("// Event handler modules\n\n");

        for event in &schema.events {
            for handler in &event.handlers {
                let handler_file = handlers_dir.join("events").join(format!("{}.rs", handler));
                if handler_file.exists() {
                    events_mod.push_str(&format!("pub mod {};\n", handler));
                }
            }
        }

        fs::write(handlers_dir.join("events").join("mod.rs"), events_mod)?;
    }

    if handlers_dir.join("websockets").exists() {
        let mut websockets_mod = String::new();
        websockets_mod.push_str("// WebSocket handler modules\n\n");

        for ws in &schema.websockets {
            let mut all_handlers = std::collections::HashSet::new();
            for handler in &ws.on_connect {
                all_handlers.insert(handler.clone());
            }
            for handler in &ws.on_message {
                all_handlers.insert(handler.clone());
            }
            for handler in &ws.on_disconnect {
                all_handlers.insert(handler.clone());
            }

            for handler in all_handlers {
                let handler_file = handlers_dir.join("websockets").join(format!("{}.rs", handler));
                if handler_file.exists() {
                    websockets_mod.push_str(&format!("pub mod {};\n", handler));
                }
            }
        }

        fs::write(handlers_dir.join("websockets").join("mod.rs"), websockets_mod)?;
    }

    if middlewares_dir.exists() {
        let mut middlewares_mod = String::new();
        middlewares_mod.push_str("// Middleware handler modules\n\n");

        let mut middleware_names = std::collections::HashSet::new();
        for api in &schema.apis {
            for mw in &api.middlewares {
                middleware_names.insert(mw.clone());
            }
        }
        for ws in &schema.websockets {
            for mw in &ws.middlewares {
                middleware_names.insert(mw.clone());
            }
        }

        for mw_name in middleware_names {
            let mw_snake = templates::to_snake_case(&mw_name);
            let handler_file = middlewares_dir.join(format!("{}.rs", mw_snake));
            if handler_file.exists() {
                middlewares_mod.push_str(&format!("pub mod {};\n", mw_snake));
            }
        }

        fs::write(middlewares_dir.join("mod.rs"), middlewares_mod)?;
    }

    Ok(())
}

fn generate_handlers_registration(schema: &Schema, output_dir: &Path) -> Result<()> {
    let generated_dir = output_dir.join("generated");
    let handlers_dir = output_dir.join("handlers");

    let mut content = String::new();
    content.push_str("// Auto-generated handler registration\n");
    content.push_str("// DO NOT EDIT MANUALLY\n\n");

    content.push_str("use rohas_runtime::{RustRuntime, HandlerContext, HandlerResult, Result};\n");
    content.push_str("use std::sync::Arc;\n");
    content.push_str("use std::sync::OnceLock;\n\n");

    content.push_str("// Global registry for automatic handler registration\n");
    content.push_str("static RUNTIME_REGISTRY: OnceLock<Arc<RustRuntime>> = OnceLock::new();\n\n");
    content.push_str("/// Set the runtime for automatic handler registration.\n");
    content.push_str("/// This is called automatically by the engine.\n");
    content.push_str("/// This function is public so it can be called from the engine.\n");
    content.push_str("/// Note: Each dylib has its own OnceLock, so this can be called fresh on each reload.\n");
    content.push_str("pub fn set_runtime(runtime: Arc<RustRuntime>) {\n");
    content.push_str("    // Set the runtime (this will only succeed once per dylib load, which is what we want)\n");
    content.push_str("    let _ = RUNTIME_REGISTRY.set(runtime);\n");
    content.push_str("    // Always trigger registration (important for hot reload)\n");
    content.push_str("    register_all_handlers_internal().expect(\"Failed to register handlers\");\n");
    content.push_str("}\n\n");

    let mut has_handlers = false;

    for api in &schema.apis {
        let handler_name = templates::to_snake_case(&api.name);
        let handler_file = handlers_dir.join("api").join(format!("{}.rs", handler_name));
        if handler_file.exists() {
            has_handlers = true;
            break;
        }
    }

    if !has_handlers {
        for event in &schema.events {
            for handler in &event.handlers {
                let handler_file = handlers_dir.join("events").join(format!("{}.rs", handler));
                if handler_file.exists() {
                    has_handlers = true;
                    break;
                }
            }
            if has_handlers {
                break;
            }
        }
    }

    if !has_handlers {
        content.push_str("/// Register all handlers with the Rust runtime.\n");
        content.push_str("/// No handlers found - implement handlers in src/handlers/ to register them.\n");
        content.push_str("pub async fn register_all_handlers(_runtime: Arc<RustRuntime>) -> Result<()> {\n");
        content.push_str("    Ok(())\n");
        content.push_str("}\n\n");
        content.push_str("fn register_all_handlers_internal() -> Result<()> {\n");
        content.push_str("    Ok(())\n");
        content.push_str("}\n");
        fs::write(generated_dir.join("handlers.rs"), content)?;
        return Ok(());
    }

    content.push_str("// Import handler functions\n");

    for api in &schema.apis {
        let handler_name = templates::to_snake_case(&api.name);
        let handler_file = handlers_dir.join("api").join(format!("{}.rs", handler_name));

        if handler_file.exists() {
            content.push_str(&format!(
                "use crate::handlers::api::{}::handle_{};\n",
                handler_name, handler_name
            ));
        }
    }

    for event in &schema.events {
        for handler in &event.handlers {
            let handler_file = handlers_dir.join("events").join(format!("{}.rs", handler));

            if handler_file.exists() {
                content.push_str(&format!(
                    "use crate::handlers::events::{}::{};\n",
                    handler, handler
                ));
            }
        }
    }

    let websockets_handlers_dir = output_dir.join("handlers/websockets");
    for ws in &schema.websockets {
        for handler in &ws.on_connect {
            let handler_file = websockets_handlers_dir.join(format!("{}.rs", handler));
            if handler_file.exists() {
                content.push_str(&format!(
                    "use crate::handlers::websockets::{}::{};\n",
                    handler, handler
                ));
            }
        }
        for handler in &ws.on_message {
            let handler_file = websockets_handlers_dir.join(format!("{}.rs", handler));
            if handler_file.exists() {
                content.push_str(&format!(
                    "use crate::handlers::websockets::{}::{};\n",
                    handler, handler
                ));
            }
        }
        for handler in &ws.on_disconnect {
            let handler_file = websockets_handlers_dir.join(format!("{}.rs", handler));
            if handler_file.exists() {
                content.push_str(&format!(
                    "use crate::handlers::websockets::{}::{};\n",
                    handler, handler
                ));
            }
        }
    }

    let middlewares_dir = output_dir.join("middlewares");
    let mut middleware_names = std::collections::HashSet::new();
    for api in &schema.apis {
        for mw in &api.middlewares {
            middleware_names.insert(mw.clone());
        }
    }
    for ws in &schema.websockets {
        for mw in &ws.middlewares {
            middleware_names.insert(mw.clone());
        }
    }

    for mw_name in &middleware_names {
        let mw_snake = templates::to_snake_case(mw_name);
        let handler_file = middlewares_dir.join(format!("{}.rs", mw_snake));
        if handler_file.exists() {
            let handler_fn_name = format!("{}_middleware", mw_snake);
            content.push_str(&format!(
                "use crate::middlewares::{}::{};\n",
                mw_snake, handler_fn_name
            ));
        }
    }

    content.push_str("\n");
    content.push_str("/// Register all handlers with the Rust runtime.\n");
    content.push_str("/// This function should be called during engine initialization.\n");
    content.push_str("pub async fn register_all_handlers(runtime: Arc<RustRuntime>) -> Result<()> {\n");
    content.push_str("    set_runtime(runtime);\n");
    content.push_str("    Ok(())\n");
    content.push_str("}\n\n");

    content.push_str("/// Internal registration function (synchronous, for static initialization).\n");
    content.push_str("fn register_all_handlers_internal() -> Result<()> {\n");
    content.push_str("    use tracing::info;\n");
    content.push_str("    info!(\"Registering Rust handlers from dylib...\");\n");
    content.push_str("    let runtime = RUNTIME_REGISTRY.get().ok_or_else(|| rohas_runtime::RuntimeError::ExecutionFailed(\"Runtime not set\".into()))?;\n");
    content.push_str("    let rt = tokio::runtime::Runtime::new().map_err(|e| rohas_runtime::RuntimeError::ExecutionFailed(e.to_string()))?;\n");
    content.push_str("    rt.block_on(async {\n");

    for api in &schema.apis {
        let handler_name = templates::to_snake_case(&api.name);
        let handler_file = handlers_dir.join("api").join(format!("{}.rs", handler_name));

        if handler_file.exists() {
            content.push_str(&format!(
                "        // Register API handler: {}\n",
                api.name
            ));
            content.push_str(&format!(
                "        runtime.register_handler(\n"
            ));
            content.push_str(&format!(
                "            \"{}\".to_string(),\n",
                handler_name
            ));
            content.push_str(&format!(
                "            |ctx: HandlerContext| async move {{\n"
            ));
            content.push_str(&format!(
                "                // Parse request from context\n"
            ));
            content.push_str(&format!(
                "                let req: crate::generated::api::{}::{}Request = serde_json::from_value(ctx.payload.clone())?;\n",
                handler_name, api.name
            ));
            content.push_str(&format!(
                "                let mut state = crate::generated::state::State::new(&ctx.handler_name);\n"
            ));
            content.push_str(&format!(
                "                let response = handle_{}(req, &mut state).await?;\n",
                handler_name
            ));
            content.push_str(&format!(
                "                Ok(HandlerResult::success(serde_json::to_value(response)?, 0))\n"
            ));
            content.push_str(&format!(
                "            }}\n"
            ));
            content.push_str(&format!(
                "        ).await;\n"
            ));
            content.push_str(&format!(
                "        info!(\"Registered handler: {}\");\n",
                handler_name
            ));
        }
    }

    let websockets_handlers_dir = output_dir.join("handlers/websockets");
    for ws in &schema.websockets {
        let ws_module = templates::to_snake_case(&ws.name);
        
        for handler in &ws.on_connect {
            let handler_file = websockets_handlers_dir.join(format!("{}.rs", handler));
            if handler_file.exists() {
                content.push_str(&format!(
                    "        // Register WebSocket connect handler: {}\n",
                    handler
                ));
                content.push_str(&format!(
                    "        runtime.register_handler(\n"
                ));
                content.push_str(&format!(
                    "            \"{}\".to_string(),\n",
                    handler
                ));
                content.push_str(&format!(
                    "            |ctx: HandlerContext| async move {{\n"
                ));
                content.push_str(&format!(
                    "                // Parse connection from context\n"
                ));
                content.push_str(&format!(
                    "                let connection: crate::generated::websockets::{}::{}Connection = serde_json::from_value(ctx.payload.clone())?;\n",
                    ws_module, ws.name
                ));
                content.push_str(&format!(
                    "                let mut state = crate::generated::state::State::new(&ctx.handler_name);\n"
                ));
                content.push_str(&format!(
                    "                let result = {}(connection, &mut state).await?;\n",
                    handler
                ));
                content.push_str(&format!(
                    "                Ok(result)\n"
                ));
                content.push_str(&format!(
                    "            }}\n"
                ));
                content.push_str(&format!(
                    "        ).await;\n"
                ));
                content.push_str(&format!(
                    "        info!(\"Registered WebSocket connect handler: {}\");\n",
                    handler
                ));
            }
        }
        
        for handler in &ws.on_message {
            let handler_file = websockets_handlers_dir.join(format!("{}.rs", handler));
            if handler_file.exists() {
                content.push_str(&format!(
                    "        // Register WebSocket message handler: {}\n",
                    handler
                ));
                content.push_str(&format!(
                    "        runtime.register_handler(\n"
                ));
                content.push_str(&format!(
                    "            \"{}\".to_string(),\n",
                    handler
                ));
                content.push_str(&format!(
                    "            |ctx: HandlerContext| async move {{\n"
                ));
                content.push_str(&format!(
                    "                // Parse message and connection from context payload\n"
                ));
                content.push_str(&format!(
                    "                let payload: serde_json::Value = ctx.payload.clone();\n"
                ));
                if ws.message.is_some() {
                    content.push_str(&format!(
                        "                let message: crate::generated::websockets::{}::{}Message = serde_json::from_value(payload.get(\"message\").cloned().unwrap_or(serde_json::json!({{}})))?;\n",
                        ws_module, ws.name
                    ));
                }
                content.push_str(&format!(
                    "                let connection: crate::generated::websockets::{}::{}Connection = serde_json::from_value(payload.get(\"connection\").cloned().unwrap_or(serde_json::json!({{}})))?;\n",
                    ws_module, ws.name
                ));
                content.push_str(&format!(
                    "                let mut state = crate::generated::state::State::new(&ctx.handler_name);\n"
                ));
                if ws.message.is_some() {
                    content.push_str(&format!(
                        "                let result = {}(message, connection, &mut state).await?;\n",
                        handler
                    ));
                } else {
                    content.push_str(&format!(
                        "                let result = {}(connection, &mut state).await?;\n",
                        handler
                    ));
                }
                content.push_str(&format!(
                    "                Ok(result)\n"
                ));
                content.push_str(&format!(
                    "            }}\n"
                ));
                content.push_str(&format!(
                    "        ).await;\n"
                ));
                content.push_str(&format!(
                    "        info!(\"Registered WebSocket message handler: {}\");\n",
                    handler
                ));
            }
        }
        
        for handler in &ws.on_disconnect {
            let handler_file = websockets_handlers_dir.join(format!("{}.rs", handler));
            if handler_file.exists() {
                content.push_str(&format!(
                    "        // Register WebSocket disconnect handler: {}\n",
                    handler
                ));
                content.push_str(&format!(
                    "        runtime.register_handler(\n"
                ));
                content.push_str(&format!(
                    "            \"{}\".to_string(),\n",
                    handler
                ));
                content.push_str(&format!(
                    "            |ctx: HandlerContext| async move {{\n"
                ));
                content.push_str(&format!(
                    "                // Parse connection from context\n"
                ));
                content.push_str(&format!(
                    "                let connection: crate::generated::websockets::{}::{}Connection = serde_json::from_value(ctx.payload.clone())?;\n",
                    ws_module, ws.name
                ));
                content.push_str(&format!(
                    "                let result = {}(connection).await?;\n",
                    handler
                ));
                content.push_str(&format!(
                    "                Ok(result)\n"
                ));
                content.push_str(&format!(
                    "            }}\n"
                ));
                content.push_str(&format!(
                    "        ).await;\n"
                ));
                content.push_str(&format!(
                    "        info!(\"Registered WebSocket disconnect handler: {}\");\n",
                    handler
                ));
            }
        }
    }


    let middlewares_dir = output_dir.join("middlewares");
    let mut middleware_names = std::collections::HashSet::new();
    for api in &schema.apis {
        for mw in &api.middlewares {
            middleware_names.insert(mw.clone());
        }
    }
    for ws in &schema.websockets {
        for mw in &ws.middlewares {
            middleware_names.insert(mw.clone());
        }
    }

    for mw_name in middleware_names {
        let mw_snake = templates::to_snake_case(&mw_name);
        let handler_file = middlewares_dir.join(format!("{}.rs", mw_snake));
        if handler_file.exists() {
            let handler_fn_name = format!("{}_middleware", mw_snake);
            content.push_str(&format!(
                "        // Register middleware handler: {}\n",
                mw_name
            ));
            content.push_str(&format!(
                "        runtime.register_handler(\n"
            ));
            content.push_str(&format!(
                "            \"{}\".to_string(),\n",
                mw_snake
            ));
            content.push_str(&format!(
                "            |ctx: HandlerContext| async move {{\n"
            ));
            content.push_str(&format!(
                "                let mut state = crate::generated::state::State::new(&ctx.handler_name);\n"
            ));
            content.push_str(&format!(
                "                {}(ctx, &mut state).await\n",
                handler_fn_name
            ));
            content.push_str(&format!(
                "            }}\n"
            ));
            content.push_str(&format!(
                "        ).await;\n"
            ));
            content.push_str(&format!(
                "        info!(\"Registered middleware handler: {}\");\n",
                mw_snake
            ));
        }
    }

    content.push_str("        Ok::<(), rohas_runtime::RuntimeError>(())\n");
    content.push_str("    })?;\n");
    content.push_str("    Ok(())\n");
    content.push_str("}\n");

    fs::write(generated_dir.join("handlers.rs"), content)?;
    Ok(())
}

pub fn is_in_rohas_workspace(output_dir: &Path) -> bool {
    let project_root = if output_dir.file_name().and_then(|s| s.to_str()) == Some("src") {
        output_dir.parent().unwrap_or(output_dir)
    } else {
        output_dir
    };

    let path_str = project_root.to_string_lossy();
    if path_str.contains("/examples/") || path_str.contains("\\examples\\") {
        return true;
    }

    let mut current = project_root;
    for _ in 0..5 {
        let crates_dir = current.join("crates").join("rohas-cli");
        if crates_dir.exists() {
            return true;
        }
        if let Some(parent) = current.parent() {
            current = parent;
        } else {
            break;
        }
    }

    false
}

pub fn generate_dev_scripts(output_dir: &Path) -> Result<()> {
    let project_root = if output_dir.file_name().and_then(|s| s.to_str()) == Some("src") {
        output_dir.parent().unwrap_or(output_dir).to_path_buf()
    } else {
        output_dir.to_path_buf()
    };

    let dev_script = r#"#!/bin/bash
# Development helper script for Rohas developers
# For end users: install rohas CLI and run "rohas dev --workbench" directly

set -e

# Find the workspace root (look for Cargo.toml with [workspace])
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$SCRIPT_DIR"

# Look for workspace root (go up to 10 levels to handle nested examples)
for i in {1..10}; do
    if [ -f "$WORKSPACE_ROOT/Cargo.toml" ]; then
        # Check if it's a workspace (has [workspace] and contains crates/rohas-cli)
        if grep -q "^\[workspace\]" "$WORKSPACE_ROOT/Cargo.toml" 2>/dev/null && \
            [ -d "$WORKSPACE_ROOT/crates/rohas-cli" ]; then
            break
        fi
    fi
    WORKSPACE_ROOT="$(dirname "$WORKSPACE_ROOT")"
    # Stop if we've reached the filesystem root
    if [ "$WORKSPACE_ROOT" = "/" ] || [ "$WORKSPACE_ROOT" = "$SCRIPT_DIR" ]; then
        break
    fi
done

if [ -f "$WORKSPACE_ROOT/Cargo.toml" ] && grep -q "^\[workspace\]" "$WORKSPACE_ROOT/Cargo.toml" 2>/dev/null && \
   [ -d "$WORKSPACE_ROOT/crates/rohas-cli" ]; then
    cd "$WORKSPACE_ROOT"
    REL_SCHEMA_PATH=$(python3 -c "import os; print(os.path.relpath('$SCRIPT_DIR/schema', '$WORKSPACE_ROOT'))" 2>/dev/null || \
                      perl -MFile::Spec -e "print File::Spec->abs2rel('$SCRIPT_DIR/schema', '$WORKSPACE_ROOT')" 2>/dev/null || \
                      echo "schema")
    # Check if --schema argument is already provided
    HAS_SCHEMA_ARG=false
    for arg in "$@"; do
        if [[ "$arg" == "--schema" ]] || [[ "$arg" == "-s" ]]; then
            HAS_SCHEMA_ARG=true
            break
        fi
    done
    # If no schema arg provided, add it
    if [ "$HAS_SCHEMA_ARG" = false ]; then
        exec cargo run -p rohas-cli -- dev --schema "$REL_SCHEMA_PATH" "$@"
    else
        exec cargo run -p rohas-cli -- dev "$@"
    fi
else
    # Not in workspace - try installed binary or show helpful error
    if command -v rohas >/dev/null 2>&1; then
        cd "$SCRIPT_DIR"
        exec rohas dev "$@"
    else
        echo "Error: Could not find Rohas workspace root and rohas CLI is not installed"
        echo ""
        echo "For Rohas developers: Run this script from within the rohas workspace"
        echo "For end users: Install rohas CLI first:"
        echo "  cargo install --path <path-to-rohas>/crates/rohas-cli"
        echo "  Then run: rohas dev --workbench"
        exit 1
    fi
fi
"#;

    let dev_script_path = project_root.join("dev.sh");
    fs::write(&dev_script_path, dev_script)?;

    // Make it executable (Unix-like systems)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&dev_script_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&dev_script_path, perms)?;
    }

    let makefile_content = r#"# Makefile for Rohas developers working in examples
# End users: Install rohas CLI and use "rohas dev --workbench" directly

.PHONY: dev dev-watch codegen check build validate

# Run development server (for Rohas developers - finds workspace automatically)
# Usage: make dev ARGS="--workbench"
dev:
	@./dev.sh $(ARGS)

# Run development server with workbench
dev-watch:
	@./dev.sh --workbench

# Generate code from schema (for Rohas developers)
codegen:
	@SCRIPT_DIR=$$(pwd); \
	WORKSPACE_ROOT=$$SCRIPT_DIR; \
	for i in {1..10}; do \
		if [ -f "$$WORKSPACE_ROOT/Cargo.toml" ] && grep -q "^\[workspace\]" "$$WORKSPACE_ROOT/Cargo.toml" 2>/dev/null && [ -d "$$WORKSPACE_ROOT/crates/rohas-cli" ]; then \
			break; \
		fi; \
		WORKSPACE_ROOT=$$(dirname "$$WORKSPACE_ROOT"); \
	done; \
	cd "$$WORKSPACE_ROOT" && cargo run -p rohas-cli -- codegen --schema "$$SCRIPT_DIR/schema" --output "$$SCRIPT_DIR/src" --lang rust

# Check Rust code
check:
	@CARGO_TARGET_DIR=../../target cargo check

# Build Rust project
build:
	@CARGO_TARGET_DIR=../../target cargo build --release

# Validate schema (for Rohas developers)
validate:
	@SCRIPT_DIR=$$(pwd); \
	WORKSPACE_ROOT=$$SCRIPT_DIR; \
	for i in {1..10}; do \
		if [ -f "$$WORKSPACE_ROOT/Cargo.toml" ] && grep -q "^\[workspace\]" "$$WORKSPACE_ROOT/Cargo.toml" 2>/dev/null && [ -d "$$WORKSPACE_ROOT/crates/rohas-cli" ]; then \
			break; \
		fi; \
		WORKSPACE_ROOT=$$(dirname "$$WORKSPACE_ROOT"); \
	done; \
	cd "$$WORKSPACE_ROOT" && cargo run -p rohas-cli -- validate --schema "$$SCRIPT_DIR/schema"
"#;

    fs::write(project_root.join("Makefile"), makefile_content)?;

    Ok(())
}

