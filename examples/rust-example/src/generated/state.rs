use serde_json::Value;
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
