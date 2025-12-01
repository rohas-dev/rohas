use crate::ast::*;
use crate::error::{ParseError, Result};
use crate::grammar::{RohasParser, Rule};
use pest::Parser as PestParser;
use std::fs;
use std::path::Path;
use tracing::{debug, info};

pub struct Parser;

impl Parser {
    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<Schema> {
        let path = path.as_ref();
        info!("Parsing schema file: {}", path.display());

        let content = fs::read_to_string(path)
            .map_err(|e| ParseError::FileNotFound(format!("{}: {}", path.display(), e)))?;

        Self::parse_string(&content)
    }

    pub fn parse_string(input: &str) -> Result<Schema> {
        let pairs = RohasParser::parse(Rule::schema, input)?;
        let mut schema = Schema::new();

        for pair in pairs {
            if pair.as_rule() == Rule::schema {
                for inner_pair in pair.into_inner() {
                    match inner_pair.as_rule() {
                        Rule::model => {
                            let model = Self::parse_model(inner_pair)?;
                            schema.models.push(model);
                        }
                        Rule::api => {
                            let api = Self::parse_api(inner_pair)?;
                            schema.apis.push(api);
                        }
                        Rule::event => {
                            let event = Self::parse_event(inner_pair)?;
                            schema.events.push(event);
                        }
                        Rule::cron => {
                            let cron = Self::parse_cron(inner_pair)?;
                            schema.crons.push(cron);
                        }
                        Rule::input => {
                            let input = Self::parse_input(inner_pair)?;
                            schema.inputs.push(input);
                        }
                        Rule::ws => {
                            let ws = Self::parse_websocket(inner_pair)?;
                            schema.websockets.push(ws);
                        }
                        Rule::EOI => {}
                        _ => {
                            debug!("Unexpected rule: {:?}", inner_pair.as_rule());
                        }
                    }
                }
            }
        }

        schema.validate()?;
        Ok(schema)
    }

    fn parse_model(pair: pest::iterators::Pair<Rule>) -> Result<Model> {
        let mut inner = pair.into_inner();
        let name = inner
            .next()
            .ok_or_else(|| ParseError::InvalidModel("Missing model name".into()))?
            .as_str()
            .to_string();

        let mut fields = Vec::new();

        for field_pair in inner {
            if field_pair.as_rule() == Rule::field {
                fields.push(Self::parse_field(field_pair)?);
            }
        }

        Ok(Model {
            name,
            fields,
            attributes: Vec::new(),
        })
    }

    fn parse_field(pair: pest::iterators::Pair<Rule>) -> Result<Field> {
        let mut inner = pair.into_inner();

        let name = inner
            .next()
            .ok_or_else(|| ParseError::InvalidModel("Missing field name".into()))?
            .as_str()
            .to_string();

        let field_type_pair = inner
            .next()
            .ok_or_else(|| ParseError::InvalidModel("Missing field type".into()))?;

        let field_type = Self::parse_field_type(field_type_pair)?;

        let mut optional = false;
        let mut attributes = Vec::new();

        for item in inner {
            match item.as_rule() {
                Rule::optional => optional = true,
                Rule::attribute => attributes.push(Self::parse_attribute(item)?),
                _ => {}
            }
        }

        Ok(Field {
            name,
            field_type,
            optional,
            attributes,
        })
    }

    fn parse_field_type(pair: pest::iterators::Pair<Rule>) -> Result<FieldType> {
        let mut inner = pair.into_inner();
        let type_name = inner
            .next()
            .ok_or_else(|| ParseError::InvalidType("Missing type name".into()))?
            .as_str();

        let mut field_type = FieldType::from_str(type_name);

        // Check for array suffix
        if let Some(array_pair) = inner.next() {
            if array_pair.as_rule() == Rule::array_suffix {
                field_type = FieldType::Array(Box::new(field_type));
            }
        }

        Ok(field_type)
    }

    fn parse_attribute(pair: pest::iterators::Pair<Rule>) -> Result<Attribute> {
        let mut inner = pair.into_inner();
        let name = inner
            .next()
            .ok_or_else(|| ParseError::InvalidAttribute("Missing attribute name".into()))?
            .as_str()
            .to_string();

        let mut args = Vec::new();

        for arg_pair in inner {
            if arg_pair.as_rule() == Rule::attr_args {
                for arg in arg_pair.into_inner() {
                    if arg.as_rule() == Rule::attr_arg_list {
                        for item in arg.into_inner() {
                            args.push(item.as_str().trim_matches('"').to_string());
                        }
                    }
                }
            }
        }

        Ok(Attribute { name, args })
    }

    fn parse_api(pair: pest::iterators::Pair<Rule>) -> Result<Api> {
        let mut inner = pair.into_inner();
        let name = inner
            .next()
            .ok_or_else(|| ParseError::InvalidApi("Missing API name".into()))?
            .as_str()
            .to_string();

        let mut method = None;
        let mut path = None;
        let mut body = None;
        let mut response = None;
        let mut triggers = Vec::new();
        let mut middlewares = Vec::new();

        for prop in inner {
            if prop.as_rule() == Rule::api_property {
                let prop_text = prop.as_str();
                let mut prop_inner = prop.into_inner();

                if let Some(key) = prop_inner.next() {
                    match key.as_rule() {
                        Rule::http_method => method = HttpMethod::from_str(key.as_str()),
                        Rule::string => path = Some(key.as_str().trim_matches('"').to_string()),
                        Rule::ident => {
                            if prop_text.starts_with("body:") {
                                body = Some(key.as_str().to_string());
                            } else if prop_text.starts_with("response:") {
                                response = Some(key.as_str().to_string());
                            }
                        }
                        Rule::trigger_list => {
                            triggers = Self::parse_string_list(key)?;
                        }
                        Rule::string_list | Rule::middleware_list => {
                            middlewares = Self::parse_string_list(key)?;
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(Api {
            name,
            method: method.ok_or_else(|| ParseError::InvalidApi("Missing HTTP method".into()))?,
            path: path.ok_or_else(|| ParseError::InvalidApi("Missing path".into()))?,
            body,
            response: response.ok_or_else(|| ParseError::InvalidApi("Missing response".into()))?,
            triggers,
            middlewares,
        })
    }

    fn parse_event(pair: pest::iterators::Pair<Rule>) -> Result<Event> {
        let mut inner = pair.into_inner();
        let name = inner
            .next()
            .ok_or_else(|| ParseError::InvalidEvent("Missing event name".into()))?
            .as_str()
            .to_string();

        let mut payload = String::new();
        let mut handlers = Vec::new();
        let mut triggers = Vec::new();
        let mut adapter_type = None;

        for prop in inner {
            if prop.as_rule() == Rule::event_property {
                let prop_text = prop.as_str();
                let mut prop_inner = prop.into_inner();
                if let Some(value) = prop_inner.next() {
                    match value.as_rule() {
                        Rule::ident => {
                            if prop_text.starts_with("payload:") {
                                payload = value.as_str().to_string();
                            } else if prop_text.starts_with("type:") {
                                let type_str = value.as_str().to_lowercase();
                                if type_str == "sqs" || type_str == "eventbridge" {
                                    adapter_type = Some(type_str);
                                } else {
                                    return Err(ParseError::InvalidEvent(format!(
                                        "Invalid adapter type '{}' for event '{}'. Must be 'sqs' or 'eventbridge'",
                                        value.as_str(), name
                                    )));
                                }
                            }
                        }
                        Rule::handler_list | Rule::trigger_list => {
                            let items = Self::parse_string_list(value)?;
                            if prop_text.starts_with("handler:") {
                                handlers = items;
                            } else if prop_text.starts_with("triggers:") {
                                triggers = items;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(Event {
            name,
            payload,
            handlers,
            triggers,
            adapter_type,
        })
    }

    fn parse_cron(pair: pest::iterators::Pair<Rule>) -> Result<Cron> {
        let mut inner = pair.into_inner();
        let name = inner
            .next()
            .ok_or_else(|| ParseError::InvalidCron("Missing cron name".into()))?
            .as_str()
            .to_string();

        let mut schedule = String::new();
        let mut triggers = Vec::new();

        for prop in inner {
            if prop.as_rule() == Rule::cron_property {
                let mut prop_inner = prop.into_inner();
                if let Some(value) = prop_inner.next() {
                    match value.as_rule() {
                        Rule::string => schedule = value.as_str().trim_matches('"').to_string(),
                        Rule::trigger_list => triggers = Self::parse_string_list(value)?,
                        _ => {}
                    }
                }
            }
        }

        Ok(Cron {
            name,
            schedule,
            triggers,
        })
    }

    fn parse_input(pair: pest::iterators::Pair<Rule>) -> Result<Input> {
        let mut inner = pair.into_inner();
        let name = inner
            .next()
            .ok_or_else(|| ParseError::InvalidModel("Missing input name".into()))?
            .as_str()
            .to_string();

        let mut fields = Vec::new();

        for field_pair in inner {
            if field_pair.as_rule() == Rule::input_field {
                let mut field_inner = field_pair.into_inner();

                let field_name = field_inner
                    .next()
                    .ok_or_else(|| ParseError::InvalidModel("Missing field name".into()))?
                    .as_str()
                    .to_string();

                let field_type_pair = field_inner
                    .next()
                    .ok_or_else(|| ParseError::InvalidModel("Missing field type".into()))?;

                let field_type = Self::parse_field_type(field_type_pair)?;

                let optional = field_inner.next().is_some();

                fields.push(Field {
                    name: field_name,
                    field_type,
                    optional,
                    attributes: Vec::new(),
                });
            }
        }

        Ok(Input { name, fields })
    }

    fn parse_string_list(pair: pest::iterators::Pair<Rule>) -> Result<Vec<String>> {
        let mut items = Vec::new();
        for item in pair.into_inner() {
            match item.as_rule() {
                Rule::ident => items.push(item.as_str().to_string()),
                Rule::string => items.push(item.as_str().trim_matches('"').to_string()),
                _ => {}
            }
        }
        Ok(items)
    }

    fn parse_websocket(pair: pest::iterators::Pair<Rule>) -> Result<WebSocket> {
        let mut inner = pair.into_inner();
        let name = inner
            .next()
            .ok_or_else(|| ParseError::InvalidApi("Missing websocket name".into()))?
            .as_str()
            .to_string();

        let mut path = None;
        let mut message = None;
        let mut on_connect = Vec::new();
        let mut on_message = Vec::new();
        let mut on_disconnect = Vec::new();
        let mut triggers = Vec::new();
        let mut broadcast = false;
        let mut middlewares = Vec::new();

        for prop in inner {
            if prop.as_rule() == Rule::ws_property {
                let prop_text = prop.as_str();
                let mut prop_inner = prop.into_inner();

                if let Some(key) = prop_inner.next() {
                    match key.as_rule() {
                        Rule::string => {
                            if prop_text.starts_with("path:") {
                                path = Some(key.as_str().trim_matches('"').to_string());
                            }
                        }
                        Rule::ident => {
                            if prop_text.starts_with("message:") {
                                message = Some(key.as_str().to_string());
                            }
                        }
                        Rule::handler_list => {
                            if prop_text.starts_with("onConnect:") {
                                on_connect = Self::parse_string_list(key)?;
                            } else if prop_text.starts_with("onMessage:") {
                                on_message = Self::parse_string_list(key)?;
                            } else if prop_text.starts_with("onDisconnect:") {
                                on_disconnect = Self::parse_string_list(key)?;
                            }
                        }
                        Rule::trigger_list => {
                            triggers = Self::parse_string_list(key)?;
                        }
                        Rule::string_list | Rule::middleware_list => {
                            if prop_text.starts_with("middlewares:") {
                                middlewares = Self::parse_string_list(key)?;
                            }
                        }
                        Rule::boolean => {
                            if prop_text.starts_with("broadcast:") {
                                broadcast = key.as_str() == "true";
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(WebSocket {
            name,
            path: path.ok_or_else(|| ParseError::InvalidApi("Missing path".into()))?,
            message,
            on_connect,
            on_message,
            on_disconnect,
            triggers,
            broadcast,
            middlewares,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_model() {
        let input = r#"
            model User {
                id Int @id @auto
                name String
                email String @unique
            }
        "#;

        let schema = Parser::parse_string(input).expect("Failed to parse");
        assert_eq!(schema.models.len(), 1);
        assert_eq!(schema.models[0].name, "User");
        assert_eq!(schema.models[0].fields.len(), 3);
    }

    #[test]
    fn test_parse_api() {
        let input = r#"
            api CreateUser {
                method: POST
                path: "/users"
                body: CreateUserInput
                response: User
                triggers: [UserCreated]
            }
        "#;

        let schema = Parser::parse_string(input).expect("Failed to parse");
        assert_eq!(schema.apis.len(), 1);
        assert_eq!(schema.apis[0].name, "CreateUser");
    }

    #[test]
    fn test_parse_event() {
        let input = r#"
            event UserCreated {
                payload: User
                handler: [send_welcome_email, update_analytics]
                triggers: [NotifyAdmin]
            }
        "#;

        let schema = Parser::parse_string(input).expect("Failed to parse");
        assert_eq!(schema.events.len(), 1);
        assert_eq!(schema.events[0].name, "UserCreated");
        assert_eq!(schema.events[0].handlers.len(), 2);
    }
}
