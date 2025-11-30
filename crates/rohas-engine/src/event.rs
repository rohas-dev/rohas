use crate::error::{EngineError, Result};
use crate::trace::{TraceEntryType, TraceStatus, TriggeredEventInfo};
use crate::telemetry::TraceStore;
use adapter_memory::MemoryAdapter;
use rohas_parser::{Event as SchemaEvent, Schema};
use rohas_runtime::Executor;
use std::sync::Arc;
use tracing::{debug, info};

pub struct EventBus {
    adapter: Arc<MemoryAdapter>,
    executor: Arc<Executor>,
    schema: Arc<Schema>,
    trace_store: Arc<TraceStore>,
}

impl EventBus {
    pub fn new(
        adapter: Arc<MemoryAdapter>,
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

        for event in &self.schema.events {
            self.subscribe_event(event).await?;
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

        debug!("Subscribing to event: {}", event_name);

        self.adapter
            .subscribe_fn(event_name.clone(), move |msg| {
                let handlers = handlers.clone();
                let triggers = triggers.clone();
                let executor = executor.clone();
                let adapter = adapter.clone();
                let event_name = event_name.clone();
                let event_payload_type = event_payload_type.clone();
                let trace_store = trace_store.clone();

                async move {
                    let span = tracing::info_span!(
                        "event_processing",
                        event = %event_name,
                    );
                    let _enter = span.enter();
                    
                    info!("Received event: {}", event_name);

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
                        
                        debug!("Executing handler: {}", handler_name);

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
                        debug!("Triggering downstream event: {}", trigger);
                        let trigger_start = std::time::Instant::now();
                        let publish_result = adapter.publish(trigger, msg.payload.clone()).await;
                        let trigger_duration = trigger_start.elapsed().as_millis() as u64;
                        let trigger_timestamp = chrono::Utc::now().to_rfc3339();

                        if let Err(e) = publish_result {
                            tracing::error!("Failed to trigger event {}: {}", trigger, e);
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
            })
            .await?;

        Ok(())
    }

    pub async fn emit(
        &self,
        event_name: impl Into<String>,
        payload: serde_json::Value,
    ) -> Result<()> {
        let event_name = event_name.into();
        debug!("Emitting event: {}", event_name);

        self.adapter
            .publish(event_name.clone(), payload)
            .await
            .map_err(|e| EngineError::EventDispatch(format!("Failed to emit event: {}", e)))?;

        Ok(())
    }
}
