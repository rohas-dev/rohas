use crate::adapter::Adapter;
use crate::error::{EngineError, Result};
use crate::trace::{TraceEntryType, TraceStatus, TriggeredEventInfo};
use crate::telemetry::TraceStore;
use rohas_parser::{Event as SchemaEvent, Schema};
use rohas_runtime::Executor;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

pub struct EventBus {
    adapter: Arc<Adapter>,
    executor: Arc<Executor>,
    schema: Arc<Schema>,
    trace_store: Arc<TraceStore>,
}

impl EventBus {
    pub fn new(
        adapter: Arc<Adapter>,
        executor: Arc<Executor>,
        schema: Arc<Schema>,
        trace_store: Arc<TraceStore>,
    ) -> Self {
        Self {
            adapter,
            executor,
            schema,
            trace_store,
        }
    }

    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing event bus");
        info!("Total events in schema: {}", self.schema.events.len());

        for event in &self.schema.events {
            info!("Processing event: {} (adapter_type: {:?})", event.name, event.adapter_type);
            match self.subscribe_event(event).await {
                Ok(_) => {
                    info!("Successfully subscribed to event: {}", event.name);
                }
                Err(e) => {
                    error!("Failed to subscribe to event '{}': {}", event.name, e);
                    return Err(e);
                }
            }
        }

        info!(
            "Event bus initialized with {} events",
            self.schema.events.len()
        );
        Ok(())
    }

    async fn subscribe_event(&self, event: &SchemaEvent) -> Result<()> {
        let event_name = event.name.clone();
        let handlers = event.handlers.clone();
        let triggers = event.triggers.clone();
        let event_payload_type = event.payload.clone();
        let executor = self.executor.clone();
        let adapter = self.adapter.clone();
        let trace_store = self.trace_store.clone();
        let schema = self.schema.clone();
        
        let adapter_type = event.adapter_type.as_deref();

        if let Some(adapter_type) = adapter_type {
            info!("Subscribing to event: {} (via {})", event_name, adapter_type);
        } else {
            debug!("Subscribing to event: {}", event_name);
        }

        let adapter_type_clone = adapter_type;
        self.adapter
            .subscribe_with_type(event_name.clone(), move |msg: adapter_memory::Message| {
                let handlers = handlers.clone();
                let triggers = triggers.clone();
                let executor = executor.clone();
                let adapter = adapter.clone();
                let event_name = event_name.clone();
                let event_payload_type = event_payload_type.clone();
                let trace_store = trace_store.clone();
                let schema = schema.clone();

                async move {
                    let span = tracing::info_span!(
                        "event_processing",
                        event = %event_name,
                    );
                    let _enter = span.enter();
                    
                    info!("=== Received event: {} ===", event_name);
                    info!("Event payload: {:?}", msg.payload);
                    info!("Event handlers to execute: {:?}", handlers);

                    let mut metadata = std::collections::HashMap::new();
                    metadata.insert("event".to_string(), event_name.clone());
                    let trace_id = trace_store
                        .start_trace(event_name.clone(), TraceEntryType::Event, metadata)
                        .await;

                    let mut any_handler_failed = false;
                    let mut first_error: Option<String> = None;

                    for handler_name in &handlers {
                        let handler_span = tracing::info_span!(
                            "event_handler",
                            handler = %handler_name,
                            event = %event_name,
                        );
                        let _enter = handler_span.enter();
                        
                        info!("Executing handler: {} for event: {}", handler_name, event_name);

                        let mut handler_context =
                            rohas_runtime::HandlerContext::new(handler_name, msg.payload.clone());
                        handler_context = handler_context.with_metadata("event_name", &event_name);
                        handler_context = handler_context
                            .with_metadata("event_payload_type", &event_payload_type);

                        let start = std::time::Instant::now();
                        let result = executor.execute_with_context(handler_context).await;
                        let duration_ms = start.elapsed().as_millis() as u64;

                        match &result {
                            Ok(exec_result) => {
                                trace_store
                                    .add_step(
                                        &trace_id,
                                        handler_name.clone(),
                                        duration_ms.max(exec_result.execution_time_ms),
                                        exec_result.success,
                                        exec_result.error.clone(),
                                    )
                                    .await;

                                if exec_result.success {
                                    info!("Handler {} completed successfully", handler_name);
                                } else {
                                    any_handler_failed = true;
                                    if first_error.is_none() {
                                        first_error = exec_result.error.clone();
                                    }
                                    tracing::error!(
                                        "Handler {} failed: {:?}",
                                        handler_name,
                                        exec_result.error
                                    );
                                }
                            }
                            Err(e) => {
                                any_handler_failed = true;
                                let err_msg = e.to_string();
                                if first_error.is_none() {
                                    first_error = Some(err_msg.clone());
                                }
                                tracing::error!(
                                    "Failed to execute handler {}: {}",
                                    handler_name,
                                    e
                                );

                                trace_store
                                    .add_step(
                                        &trace_id,
                                        handler_name.clone(),
                                        duration_ms,
                                        false,
                                        Some(err_msg),
                                    )
                                    .await;
                            }
                        }
                    }

                    let mut triggered_events: Vec<TriggeredEventInfo> = Vec::new();
                    for trigger in &triggers {
                        info!("Triggering downstream event: {}", trigger);
                        let trigger_start = std::time::Instant::now();
                        let trigger_event = schema.events.iter().find(|e| e.name == *trigger);
                        let adapter_type = trigger_event.and_then(|e| e.adapter_type.as_deref());
                        let publish_result = adapter.publish_with_type(trigger, msg.payload.clone(), adapter_type).await;
                        let trigger_duration = trigger_start.elapsed().as_millis() as u64;
                        let trigger_timestamp = chrono::Utc::now().to_rfc3339();

                        match publish_result {
                            Ok(_) => {
                                if let Some(adapter_type) = adapter_type {
                                    info!("Successfully triggered event: {} (via {})", trigger, adapter_type);
                                } else {
                                    info!("Successfully triggered event: {}", trigger);
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to trigger event {}: {}", trigger, e);
                            }
                        }

                        triggered_events.push(TriggeredEventInfo {
                            event_name: trigger.clone(),
                            timestamp: trigger_timestamp,
                            duration_ms: trigger_duration,
                        });
                    }

                    if !triggered_events.is_empty() {
                        trace_store
                            .add_step_with_triggers(
                                &trace_id,
                                format!("{} triggers", event_name),
                                triggered_events
                                    .iter()
                                    .map(|t| t.duration_ms)
                                    .sum(),
                                true,
                                None,
                                triggered_events,
                            )
                            .await;
                    }

                    let status = if any_handler_failed {
                        TraceStatus::Failed
                    } else {
                        TraceStatus::Success
                    };
                    trace_store
                        .complete_trace(&trace_id, status, first_error)
                        .await;

                    Ok(())
                }
            }, adapter_type_clone)
            .await?;

        Ok(())
    }

    pub async fn emit(
        &self,
        event_name: impl Into<String>,
        payload: serde_json::Value,
    ) -> Result<()> {
        let event_name = event_name.into();
        info!("Emitting event: {}", event_name);

        let event = self.schema.events.iter().find(|e| e.name == event_name);
        let adapter_type = event.and_then(|e| e.adapter_type.as_deref());

        match self.adapter.publish_with_type(event_name.clone(), payload, adapter_type).await {
            Ok(_) => {
                if let Some(adapter_type) = adapter_type {
                    info!("Successfully emitted event: {} (via {})", event_name, adapter_type);
                } else {
                    info!("Successfully emitted event: {}", event_name);
                }
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to emit event {}: {}", event_name, e);
                Err(EngineError::EventDispatch(format!("Failed to emit event: {}", e)))
            }
        }
    }
}
