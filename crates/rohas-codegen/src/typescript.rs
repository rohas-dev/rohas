use crate::error::Result;
use crate::templates;
use rohas_parser::{Api, Event, Model, Schema};
use std::fs;
use std::path::Path;

pub fn generate_models(schema: &Schema, output_dir: &Path) -> Result<()> {
    let models_dir = output_dir.join("generated/models");

    for model in &schema.models {
        let content = generate_model_content(model);
        let file_name = format!("{}.ts", templates::to_snake_case(&model.name));
        fs::write(models_dir.join(file_name), content)?;
    }

    Ok(())
}

fn generate_model_content(model: &Model) -> String {
    let mut content = String::new();

    content.push_str("import { z } from 'zod';\n\n");

    content.push_str(&format!("export interface {} {{\n", model.name));

    for field in &model.fields {
        let ts_type = field.field_type.to_typescript();
        let optional = if field.optional { "?" } else { "" };
        content.push_str(&format!("  {}{}: {};\n", field.name, optional, ts_type));
    }

    content.push_str("}\n\n");

    // Generate zod schema
    content.push_str(&format!("export const {}Schema = z.object({{\n", model.name));
    for field in &model.fields {
        let zod_type = field_type_to_zod(&field.field_type, field.optional);
        content.push_str(&format!("  {}: {},\n", field.name, zod_type));
    }
    content.push_str("});\n\n");

    content.push_str(&format!(
        "export function is{}(obj: any): obj is {} {{\n",
        model.name, model.name
    ));
    content.push_str(&format!("  return {}Schema.safeParse(obj).success;\n", model.name));
    content.push_str("}\n");

    content
}

fn field_type_to_zod(field_type: &rohas_parser::FieldType, optional: bool) -> String {
    use rohas_parser::FieldType;
    
    let zod_type = match field_type {
        FieldType::Int | FieldType::Float => "z.number()".to_string(),
        FieldType::String => "z.string()".to_string(),
        FieldType::Boolean => "z.boolean()".to_string(),
        FieldType::DateTime => "z.date()".to_string(),
        FieldType::Json => "z.any()".to_string(),
        FieldType::Custom(name) => format!("{}Schema", name),
        FieldType::Array(inner) => {
            let inner_zod = field_type_to_zod(inner, false);
            format!("z.array({})", inner_zod)
        }
    };

    if optional {
        format!("{}.optional()", zod_type)
    } else {
        zod_type
    }
}


pub fn generate_dtos(schema: &Schema, output_dir: &Path) -> Result<()> {
    let dto_dir = output_dir.join("generated/dto");

    for input in &schema.inputs {
        let content = generate_model_content(&rohas_parser::Model {
            name: input.name.clone(),
            fields: input.fields.clone(),
            attributes: vec![],
        });
        let file_name = format!("{}.ts", templates::to_snake_case(&input.name));
        fs::write(dto_dir.join(file_name), content)?;
    }

    Ok(())
}

pub fn generate_apis(schema: &Schema, output_dir: &Path) -> Result<()> {
    let api_dir = output_dir.join("generated/api");

    for api in &schema.apis {
        let content = generate_api_content(api);
        let file_name = format!("{}.ts", templates::to_snake_case(&api.name));
        fs::write(api_dir.join(file_name), content)?;
    }

    let handlers_dir = output_dir.join("handlers/api");
    for api in &schema.apis {
        let file_name = format!("{}.ts", &api.name);
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

    content.push_str("import { z } from 'zod';\n");

    let request_type = format!("{}Request", api.name);
    let response_type = format!("{}Response", api.name);
    let handler_type = format!("{}Handler", api.name);

    let response_is_primitive = is_primitive_type(&api.response);

    if !response_is_primitive {
        content.push_str(&format!(
            "import {{ {}, {}Schema }} from '@generated/models/{}';\n",
            api.response,
            api.response,
            templates::to_snake_case(&api.response)
        ));
    }

    if let Some(body) = &api.body {
        let body_is_primitive = is_primitive_type(body);
        if !body_is_primitive {
            if body.ends_with("Input") {
                content.push_str(&format!(
                    "import {{ {}, {}Schema }} from '@generated/dto/{}';\n",
                    body,
                    body,
                    templates::to_snake_case(body)
                ));
            } else {
                content.push_str(&format!(
                    "import {{ {}, {}Schema }} from '@generated/models/{}';\n",
                    body,
                    body,
                    templates::to_snake_case(body)
                ));
            }
        }
    }

    if !content.is_empty() {
        content.push_str("\n");
    }

    let path_params = extract_path_params(&api.path);

    content.push_str(&format!("export interface {} {{\n", request_type));

    for param in &path_params {
        content.push_str(&format!("  {}: string;\n", param));
    }

    if let Some(body) = &api.body {
        let ts_type = if is_primitive_type(body) {
            primitive_to_typescript(body)
        } else {
            body.to_string()
        };
        content.push_str(&format!("  body: {};\n", ts_type));
    }

    content.push_str("  queryParams?: Record<string, string>;\n");

    content.push_str("}\n\n");

    // Generate zod schema for request
    content.push_str(&format!("export const {}Schema = z.object({{\n", request_type));
    for param in &path_params {
        content.push_str(&format!("  {}: z.string(),\n", param));
    }
    if let Some(body) = &api.body {
        let body_is_primitive = is_primitive_type(body);
        if body_is_primitive {
            let zod_type = match body.as_str() {
                "String" => "z.string()",
                "Int" | "Float" => "z.number()",
                "Boolean" => "z.boolean()",
                "DateTime" | "Date" => "z.date()",
                _ => "z.any()",
            };
            content.push_str(&format!("  body: {},\n", zod_type));
        } else {
            if body.ends_with("Input") {
                content.push_str(&format!("  body: {}Schema,\n", body));
            } else {
                content.push_str(&format!("  body: {}Schema,\n", body));
            }
        }
    }
    content.push_str("  queryParams: z.record(z.string()).optional(),\n");
    content.push_str("});\n\n");

    let response_ts_type = if response_is_primitive {
        primitive_to_typescript(&api.response)
    } else {
        api.response.clone()
    };

    content.push_str(&format!("export interface {} {{\n", response_type));
    content.push_str(&format!("  data: {};\n", response_ts_type));
    content.push_str("}\n\n");

    // Generate zod schema for response
    let response_zod_type = if response_is_primitive {
        match api.response.as_str() {
            "String" => "z.string()".to_string(),
            "Int" | "Float" => "z.number()".to_string(),
            "Boolean" => "z.boolean()".to_string(),
            "DateTime" | "Date" => "z.date()".to_string(),
            _ => "z.any()".to_string(),
        }
    } else {
        format!("{}Schema", api.response)
    };
    content.push_str(&format!("export const {}Schema = z.object({{\n", response_type));
    content.push_str(&format!("  data: {},\n", response_zod_type));
    content.push_str("});\n\n");

    content.push_str(&format!(
        "export type {} = (req: {}) => Promise<{}>;\n",
        handler_type, request_type, response_type
    ));

    content
}

fn is_primitive_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "String" | "Int" | "Float" | "Boolean" | "DateTime" | "Date"
    )
}

fn primitive_to_typescript(type_name: &str) -> String {
    match type_name {
        "String" => "string".to_string(),
        "Int" | "Float" => "number".to_string(),
        "Boolean" => "boolean".to_string(),
        "DateTime" | "Date" => "Date".to_string(),
        _ => type_name.to_string(),
    }
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

fn generate_api_handler_stub(api: &Api) -> String {
    let mut content = String::new();

    let request_type = format!("{}Request", api.name);
    let response_type = format!("{}Response", api.name);
    let handler_name = format!("handle{}", api.name);

    content.push_str(&format!(
        "import {{ {}, {} }} from '@generated/api/{}';\n",
        request_type,
        response_type,
        templates::to_snake_case(&api.name)
    ));
    content.push_str("import { State } from '@generated/state';\n\n");

    content.push_str(&format!(
        "export async function {}(req: {}, state: State): Promise<{}> {{\n",
        handler_name, request_type, response_type
    ));
    content.push_str("  // TODO: Implement handler logic\n");
    content.push_str("  // For auto-triggers (defined in schema triggers): use state.setPayload('EventName', {...})\n");
    content.push_str("  // For manual triggers: use state.triggerEvent('EventName', {...})\n");
    content.push_str("  throw new Error('Not implemented');\n");
    content.push_str("}\n");

    content
}

pub fn generate_events(schema: &Schema, output_dir: &Path) -> Result<()> {
    let events_dir = output_dir.join("generated/events");

    for event in &schema.events {
        let content = generate_event_content(event);
        let file_name = format!("{}.ts", templates::to_snake_case(&event.name));
        fs::write(events_dir.join(file_name), content)?;
    }

    // Generate handler stubs
    let handlers_dir = output_dir.join("handlers/events");
    for event in &schema.events {
        for handler in &event.handlers {
            let file_name = format!("{}.ts", handler);
            let handler_path = handlers_dir.join(&file_name);

            if !handler_path.exists() {
                let content = generate_event_handler_stub(event, handler);
                fs::write(handler_path, content)?;
            }
        }
    }

    Ok(())
}

fn payload_type_to_zod(type_name: &str) -> String {
    match type_name {
        "String" => "z.string()".to_string(),
        "Int" | "Float" => "z.number()".to_string(),
        "Boolean" | "Bool" => "z.boolean()".to_string(),
        "DateTime" | "Date" => "z.date()".to_string(),
        _ => format!("{}Schema", type_name),
    }
}

fn generate_event_content(event: &Event) -> String {
    let mut content = String::new();

    content.push_str("import { z } from 'zod';\n");
    
    let payload_is_primitive = is_primitive_type(&event.payload);
    
    if payload_is_primitive {
        content.push_str("\n");
    } else {
        content.push_str(&format!(
            "import {{ {}, {}Schema }} from '@generated/models/{}';\n\n",
            event.payload,
            event.payload,
            templates::to_snake_case(&event.payload)
        ));
    }

    let payload_ts_type = if payload_is_primitive {
        primitive_to_typescript(&event.payload)
    } else {
        event.payload.clone()
    };

    content.push_str(&format!("export interface {} {{\n", event.name));
    content.push_str(&format!("  payload: {};\n", payload_ts_type));
    content.push_str("  timestamp: Date;\n");
    content.push_str("}\n\n");

    // Generate zod schema for event
    let payload_zod_type = payload_type_to_zod(&event.payload);
    content.push_str(&format!("export const {}Schema = z.object({{\n", event.name));
    content.push_str(&format!("  payload: {},\n", payload_zod_type));
    content.push_str("  timestamp: z.date(),\n");
    content.push_str("});\n\n");

    content.push_str(&format!(
        "export type {}Handler = (event: {}) => Promise<void>;\n",
        event.name, event.name
    ));

    content
}

fn generate_event_handler_stub(event: &Event, handler_name: &str) -> String {
    let mut content = String::new();

    content.push_str(&format!(
        "import {{ {} }} from '@generated/events/{}';\n\n",
        event.name,
        templates::to_snake_case(&event.name)
    ));

    content.push_str(&format!(
        "export async function {}(event: {}): Promise<void> {{\n",
        handler_name, event.name
    ));
    content.push_str("  // TODO: Implement event handler\n");
    content.push_str(&format!("  console.log('Handling event:', event);\n"));
    content.push_str("}\n");

    content
}

pub fn generate_crons(schema: &Schema, output_dir: &Path) -> Result<()> {
    let cron_dir = output_dir.join("generated/cron");

    for cron in &schema.crons {
        let content = format!(
            "export interface {} {{\n  schedule: string;\n}}\n",
            cron.name
        );
        let file_name = format!("{}.ts", templates::to_snake_case(&cron.name));
        fs::write(cron_dir.join(file_name), content)?;
    }

    // Generate handler stubs
    let handlers_dir = output_dir.join("handlers/cron");
    for cron in &schema.crons {
        let file_name = format!("{}.ts", templates::to_snake_case(&cron.name));
        let handler_path = handlers_dir.join(&file_name);

        if !handler_path.exists() {
            let content = format!(
                "export async function handle{}(): Promise<void> {{\n  // TODO: Implement cron job\n  console.log('Running cron: {}');\n}}\n",
                cron.name, cron.name
            );
            fs::write(handler_path, content)?;
        }
    }

    Ok(())
}

pub fn generate_state(output_dir: &Path) -> Result<()> {
    let generated_dir = output_dir.join("generated");
    let content = r#"export interface TriggeredEvent {
  eventName: string;
  payload: any;
}

/**
 * Context object for handlers to trigger events and access runtime state.
 */
export class State {
  private triggers: TriggeredEvent[] = [];
  private autoTriggerPayloads: Map<string, any> = new Map();

  /**
   * Manually trigger an event with the given payload.
   * 
   * Use this for events that are NOT defined in the schema's triggers list.
   * 
   * @param eventName - Name of the event to trigger
   * @param payload - Event payload data (will be serialized to JSON)
   */
  triggerEvent(eventName: string, payload: any): void {
    this.triggers.push({
      eventName,
      payload,
    });
  }

  /**
   * Set the payload for an auto-triggered event.
   * 
   * Use this for events that ARE defined in the schema's triggers list.
   * The event will be automatically triggered after the handler completes,
   * using the payload you set here.
   * 
   * @param eventName - Name of the event (must match a trigger in schema)
   * @param payload - Event payload data (will be serialized to JSON)
   */
  setPayload(eventName: string, payload: any): void {
    this.autoTriggerPayloads.set(eventName, payload);
  }

  /**
   * Get all manually triggered events. Used internally by the runtime.
   */
  getTriggers(): TriggeredEvent[] {
    return [...this.triggers];
  }

  /**
   * Get payload for an auto-triggered event. Used internally by the runtime.
   */
  getAutoTriggerPayload(eventName: string): any | undefined {
    return this.autoTriggerPayloads.get(eventName);
  }

  /**
   * Get all auto-trigger payloads. Used internally by the runtime.
   */
  getAllAutoTriggerPayloads(): Map<string, any> {
    return new Map(this.autoTriggerPayloads);
  }
}
"#;

    fs::write(generated_dir.join("state.ts"), content)?;
    Ok(())
}

pub fn generate_index(schema: &Schema, output_dir: &Path) -> Result<()> {
    let mut content = String::new();

    content.push_str("export * from './state';\n\n");

    content.push_str("// Models\n");
    for model in &schema.models {
        content.push_str(&format!(
            "export * from './models/{}';\n",
            templates::to_snake_case(&model.name)
        ));
    }

    content.push_str("\n// DTOs\n");
    for input in &schema.inputs {
        content.push_str(&format!(
            "export * from './dto/{}';\n",
            templates::to_snake_case(&input.name)
        ));
    }

    content.push_str("\n// APIs\n");
    for api in &schema.apis {
        content.push_str(&format!(
            "export * from './api/{}';\n",
            templates::to_snake_case(&api.name)
        ));
    }

    content.push_str("\n// Events\n");
    for event in &schema.events {
        content.push_str(&format!(
            "export * from './events/{}';\n",
            templates::to_snake_case(&event.name)
        ));
    }

    fs::write(output_dir.join("generated/index.ts"), content)?;

    Ok(())
}
