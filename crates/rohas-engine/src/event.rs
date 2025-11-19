use crate::error::{EngineError, Result};
use adapter_memory::MemoryAdapter;
use rohas_parser::{Event as SchemaEvent, Schema};
use rohas_runtime::Executor;
use std::sync::Arc;
use tracing::{debug, info};

pub struct EventBus {
    adapter: Arc<MemoryAdapter>,
    executor: Arc<Executor>,
    schema: Arc<Schema>,
}

impl EventBus {
    pub fn new(adapter: Arc<MemoryAdapter>, executor: Arc<Executor>, schema: Arc<Schema>) -> Self {
        Self {
            adapter,
            executor,
            schema,
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
        let executor = self.executor.clone();
        let adapter = self.adapter.clone();

        debug!("Subscribing to event: {}", event_name);

        self.adapter
            .subscribe_fn(event_name.clone(), move |msg| {
                let handlers = handlers.clone();
                let triggers = triggers.clone();
                let executor = executor.clone();
                let adapter = adapter.clone();
                let event_name = event_name.clone();

                async move {
                    info!("Received event: {}", event_name);

                    for handler_name in &handlers {
                        debug!("Executing handler: {}", handler_name);

                        match executor.execute(handler_name, msg.payload.clone()).await {
                            Ok(result) => {
                                if result.success {
                                    info!("Handler {} completed successfully", handler_name);
                                } else {
                                    tracing::error!(
                                        "Handler {} failed: {:?}",
                                        handler_name,
                                        result.error
                                    );
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to execute handler {}: {}",
                                    handler_name,
                                    e
                                );
                            }
                        }
                    }

                    for trigger in &triggers {
                        debug!("Triggering downstream event: {}", trigger);
                        if let Err(e) = adapter.publish(trigger, msg.payload.clone()).await {
                            tracing::error!("Failed to trigger event {}: {}", trigger, e);
                        }
                    }

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
