use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Schema {
    pub models: Vec<Model>,
    pub apis: Vec<Api>,
    pub events: Vec<Event>,
    pub crons: Vec<Cron>,
    pub inputs: Vec<Input>,
    pub websockets: Vec<WebSocket>,
}

impl Schema {
    pub fn new() -> Self {
        Self {
            models: Vec::new(),
            apis: Vec::new(),
            events: Vec::new(),
            crons: Vec::new(),
            inputs: Vec::new(),
            websockets: Vec::new(),
        }
    }

    pub fn validate(&self) -> crate::Result<()> {
        let mut names = std::collections::HashSet::new();

        for model in &self.models {
            if !names.insert(&model.name) {
                return Err(crate::ParseError::DuplicateDefinition(format!(
                    "Model '{}'",
                    model.name
                )));
            }
        }

        for api in &self.apis {
            if !names.insert(&api.name) {
                return Err(crate::ParseError::DuplicateDefinition(format!(
                    "API '{}'",
                    api.name
                )));
            }
        }

        for event in &self.events {
            if !names.insert(&event.name) {
                return Err(crate::ParseError::DuplicateDefinition(format!(
                    "Event '{}'",
                    event.name
                )));
            }
        }

        for websocket in &self.websockets {
            if !names.insert(&websocket.name) {
                return Err(crate::ParseError::DuplicateDefinition(format!(
                    "WebSocket '{}'",
                    websocket.name
                )));
            }
        }

        Ok(())
    }
}

impl Default for Schema {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Model {
    pub name: String,
    pub fields: Vec<Field>,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Field {
    pub name: String,
    pub field_type: FieldType,
    pub optional: bool,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FieldType {
    Int,
    String,
    Boolean,
    Float,
    DateTime,
    Json,
    Custom(String),
    Array(Box<FieldType>),
}

impl FieldType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "Int" => FieldType::Int,
            "String" => FieldType::String,
            "Boolean" | "Bool" => FieldType::Boolean,
            "Float" => FieldType::Float,
            "DateTime" => FieldType::DateTime,
            "Json" => FieldType::Json,
            _ => FieldType::Custom(s.to_string()),
        }
    }

    pub fn to_typescript(&self) -> String {
        match self {
            FieldType::Int | FieldType::Float => "number".to_string(),
            FieldType::String => "string".to_string(),
            FieldType::Boolean => "boolean".to_string(),
            FieldType::DateTime => "Date".to_string(),
            FieldType::Json => "any".to_string(),
            FieldType::Custom(name) => name.clone(),
            FieldType::Array(inner) => format!("{}[]", inner.to_typescript()),
        }
    }

    pub fn to_python(&self) -> String {
        match self {
            FieldType::Int => "int".to_string(),
            FieldType::Float => "float".to_string(),
            FieldType::String => "str".to_string(),
            FieldType::Boolean => "bool".to_string(),
            FieldType::DateTime => "datetime".to_string(),
            FieldType::Json => "dict".to_string(),
            FieldType::Custom(name) => name.clone(),
            FieldType::Array(inner) => format!("list[{}]", inner.to_python()),
        }
    }
}

/// Attribute (e.g., @id, @unique, @default)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Attribute {
    pub name: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Api {
    pub name: String,
    pub method: HttpMethod,
    pub path: String,
    pub body: Option<String>,
    pub response: String,
    pub triggers: Vec<String>,
    pub middlewares: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
}

impl HttpMethod {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "GET" => Some(HttpMethod::GET),
            "POST" => Some(HttpMethod::POST),
            "PUT" => Some(HttpMethod::PUT),
            "PATCH" => Some(HttpMethod::PATCH),
            "DELETE" => Some(HttpMethod::DELETE),
            _ => None,
        }
    }
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpMethod::GET => write!(f, "GET"),
            HttpMethod::POST => write!(f, "POST"),
            HttpMethod::PUT => write!(f, "PUT"),
            HttpMethod::PATCH => write!(f, "PATCH"),
            HttpMethod::DELETE => write!(f, "DELETE"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Event {
    pub name: String,
    pub payload: String,
    pub handlers: Vec<String>,
    pub triggers: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Cron {
    pub name: String,
    pub schedule: String,
    pub triggers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Input {
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WebSocket {
    pub name: String,
    pub path: String,
    pub message: Option<String>,
    pub on_connect: Vec<String>,
    pub on_message: Vec<String>,
    pub on_disconnect: Vec<String>,
    pub triggers: Vec<String>,
    pub broadcast: bool,
    pub middlewares: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_type_to_typescript() {
        assert_eq!(FieldType::Int.to_typescript(), "number");
        assert_eq!(FieldType::String.to_typescript(), "string");
        assert_eq!(FieldType::Boolean.to_typescript(), "boolean");
        assert_eq!(
            FieldType::Array(Box::new(FieldType::String)).to_typescript(),
            "string[]"
        );
    }

    #[test]
    fn test_field_type_to_python() {
        assert_eq!(FieldType::Int.to_python(), "int");
        assert_eq!(FieldType::String.to_python(), "str");
        assert_eq!(FieldType::Boolean.to_python(), "bool");
        assert_eq!(
            FieldType::Array(Box::new(FieldType::Int)).to_python(),
            "list[int]"
        );
    }

    #[test]
    fn test_schema_validation() {
        let mut schema = Schema::new();
        schema.models.push(Model {
            name: "User".to_string(),
            fields: vec![],
            attributes: vec![],
        });

        assert!(schema.validate().is_ok());

        // Add duplicate
        schema.models.push(Model {
            name: "User".to_string(),
            fields: vec![],
            attributes: vec![],
        });

        assert!(schema.validate().is_err());
    }
}
