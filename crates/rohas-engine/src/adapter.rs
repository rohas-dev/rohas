use crate::error::Result;
use serde_json::Value;
use std::sync::Arc;

/// Enum wrapper for different adapter types
pub enum Adapter {
    Memory(Arc<adapter_memory::MemoryAdapter>),
    Aws(Arc<adapter_aws::AwsAdapter>),
}

impl Adapter {
    /// Publish a message to a topic
    pub async fn publish(&self, topic: impl Into<String>, payload: Value) -> Result<()> {
        self.publish_with_type(topic, payload, None).await
    }

    /// Publish a message to a topic with optional adapter type override
    pub async fn publish_with_type(
        &self,
        topic: impl Into<String>,
        payload: Value,
        adapter_type: Option<&str>,
    ) -> Result<()> {
        let topic_str = topic.into();
        match self {
            Adapter::Memory(adapter) => {
                tracing::debug!("Publishing to Memory adapter - topic: {}", topic_str);
                adapter.publish(topic_str, payload)
                    .await
                    .map_err(|e| crate::error::EngineError::Adapter(e.to_string()))
            }
            Adapter::Aws(adapter) => {
                let topic_clone = topic_str.clone();
                if let Some(adapter_type) = adapter_type {
                    tracing::info!("Publishing to AWS adapter (type: {}) - topic: {}", adapter_type, topic_str);
                } else {
                    tracing::info!("Publishing to AWS adapter - topic: {}", topic_str);
                }
                adapter.publish_with_type(topic_str, payload, adapter_type)
                    .await
                    .map_err(|e| {
                        tracing::error!("AWS adapter publish failed for topic {}: {}", topic_clone, e);
                        crate::error::EngineError::Adapter(e.to_string())
                    })
            }
        }
    }

    /// Subscribe to a topic with a closure handler
    pub async fn subscribe_fn<F, Fut>(&self, topic: impl Into<String>, handler: F) -> Result<()>
    where
        F: Fn(adapter_memory::Message) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        self.subscribe_with_type(topic, handler, None).await
    }

    /// Subscribe to a topic with a closure handler and optional adapter type
    pub async fn subscribe_with_type<F, Fut>(
        &self,
        topic: impl Into<String>,
        handler: F,
        adapter_type: Option<&str>,
    ) -> Result<()>
    where
        F: Fn(adapter_memory::Message) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        match self {
            Adapter::Memory(adapter) => {
                adapter.subscribe_fn(topic, move |msg| {
                    let fut = handler(msg);
                    async move {
                        fut.await.map_err(|e| {
                            adapter_memory::AdapterError::ChannelError(e.to_string())
                        })
                    }
                })
                .await
                .map_err(|e| crate::error::EngineError::Adapter(e.to_string()))
            }
            Adapter::Aws(adapter) => {
                // Convert adapter_memory::Message to adapter_aws::Message
                adapter.subscribe_with_type(topic, move |aws_msg| {
                    let fut = handler(adapter_memory::Message {
                        topic: aws_msg.topic,
                        payload: aws_msg.payload,
                        timestamp: aws_msg.timestamp,
                        metadata: aws_msg.metadata,
                    });
                    async move {
                        fut.await.map_err(|e| {
                            adapter_aws::common::AdapterError::AwsSqs(e.to_string())
                        })
                    }
                }, adapter_type)
                .await
                .map_err(|e| crate::error::EngineError::Adapter(e.to_string()))
            }
        }
    }

    /// Get list of all topics
    pub async fn list_topics(&self) -> Vec<String> {
        match self {
            Adapter::Memory(adapter) => adapter.list_topics().await,
            Adapter::Aws(adapter) => adapter.list_topics().await,
        }
    }
}
