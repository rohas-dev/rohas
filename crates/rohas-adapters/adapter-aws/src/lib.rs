pub mod sqs;
pub mod eventbridge;
pub mod common;

pub use common::{AwsConfig, Message, Result};
pub use sqs::SqsAdapter;
pub use eventbridge::EventBridgeAdapter;

use serde_json::Value;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AwsAdapterType {
    Sqs,
    EventBridge,
}

pub enum AwsAdapter {
    Sqs(Arc<SqsAdapter>),
    EventBridge(Arc<EventBridgeAdapter>),
    Both {
        sqs: Arc<SqsAdapter>,
        eventbridge: Arc<EventBridgeAdapter>,
        default_type: AwsAdapterType,
    },
}

impl AwsAdapter {
    pub async fn new(
        adapter_type: AwsAdapterType,
        config: AwsConfig,
    ) -> common::Result<Self> {
        match adapter_type {
            AwsAdapterType::Sqs => {
                let sqs_config = sqs::SqsConfig {
                    region: config.region.clone(),
                    queue_prefix: config.queue_prefix.clone(),
                    visibility_timeout_seconds: config.visibility_timeout_seconds,
                    message_retention_seconds: config.message_retention_seconds,
                    receive_wait_time_seconds: config.receive_wait_time_seconds,
                };
                Ok(AwsAdapter::Sqs(Arc::new(
                    SqsAdapter::new(sqs_config).await?
                )))
            }
            AwsAdapterType::EventBridge => {
                let eb_config = eventbridge::EventBridgeConfig {
                    region: config.region.clone(),
                    event_bus_name: config.event_bus_name.clone(),
                    source: config.source.clone(),
                };
                Ok(AwsAdapter::EventBridge(Arc::new(
                    EventBridgeAdapter::new(eb_config).await?
                )))
            }
        }
    }

    pub async fn new_with_both(
        default_type: AwsAdapterType,
        config: AwsConfig,
    ) -> common::Result<Self> {
        tracing::info!(
            "AwsAdapter::new_with_both: Initializing with default_type: {:?}, region: {}, queue_prefix: {:?}",
            default_type,
            config.region,
            config.queue_prefix
        );
        
        let sqs_config = sqs::SqsConfig {
            region: config.region.clone(),
            queue_prefix: config.queue_prefix.clone(),
            visibility_timeout_seconds: config.visibility_timeout_seconds,
            message_retention_seconds: config.message_retention_seconds,
            receive_wait_time_seconds: config.receive_wait_time_seconds,
        };
        tracing::info!("AwsAdapter::new_with_both: Creating SQS adapter...");
        let sqs_adapter = Arc::new(SqsAdapter::new(sqs_config).await.map_err(|e| {
            tracing::error!("AwsAdapter::new_with_both: Failed to create SQS adapter: {}", e);
            e
        })?);
        tracing::info!("AwsAdapter::new_with_both: SQS adapter created successfully");

        let eb_config = eventbridge::EventBridgeConfig {
            region: config.region.clone(),
            event_bus_name: config.event_bus_name.clone(),
            source: config.source.clone(),
        };
        tracing::info!("AwsAdapter::new_with_both: Creating EventBridge adapter...");
        let eb_adapter = Arc::new(EventBridgeAdapter::new(eb_config).await.map_err(|e| {
            tracing::error!("AwsAdapter::new_with_both: Failed to create EventBridge adapter: {}", e);
            e
        })?);
        tracing::info!("AwsAdapter::new_with_both: EventBridge adapter created successfully");

        tracing::info!(
            "AwsAdapter::new_with_both: Both adapters initialized successfully with default_type: {:?}",
            default_type
        );
        
        Ok(AwsAdapter::Both {
            sqs: sqs_adapter,
            eventbridge: eb_adapter,
            default_type,
        })
    }

    pub async fn publish(
        &self,
        topic: impl Into<String>,
        payload: Value,
    ) -> common::Result<()> {
        match self {
            AwsAdapter::Sqs(adapter) => adapter.publish(topic, payload).await,
            AwsAdapter::EventBridge(adapter) => adapter.publish(topic, payload).await,
            AwsAdapter::Both { sqs, eventbridge: _, default_type: _ } => {
                sqs.publish(topic, payload).await
            }
        }
    }

    pub async fn publish_with_type(
        &self,
        topic: impl Into<String>,
        payload: Value,
        adapter_type: Option<&str>,
    ) -> common::Result<()> {
        let topic_str = topic.into();
        match self {
            AwsAdapter::Sqs(adapter) => {
                tracing::info!("AwsAdapter::publish_with_type: Using SQS adapter for topic: {}", topic_str);
                adapter.publish(topic_str, payload).await
            }
            AwsAdapter::EventBridge(adapter) => {
                tracing::info!("AwsAdapter::publish_with_type: Using EventBridge adapter for topic: {}", topic_str);
                adapter.publish(topic_str, payload).await
            }
            AwsAdapter::Both { sqs, eventbridge, default_type } => {
                let use_type = adapter_type
                    .map(|s| s.to_lowercase())
                    .unwrap_or_else(|| match default_type {
                        AwsAdapterType::Sqs => "sqs".to_string(),
                        AwsAdapterType::EventBridge => "eventbridge".to_string(),
                    });
                
                tracing::info!(
                    "AwsAdapter::publish_with_type: Both mode - requested: {:?}, using: {}, topic: {}",
                    adapter_type,
                    use_type,
                    topic_str
                );
                
                match use_type.as_str() {
                    "sqs" => {
                        tracing::info!("AwsAdapter::publish_with_type: Routing to SQS for topic: {}", topic_str);
                        sqs.publish(topic_str, payload).await
                    }
                    "eventbridge" => {
                        tracing::info!("AwsAdapter::publish_with_type: Routing to EventBridge for topic: {}", topic_str);
                        eventbridge.publish(topic_str, payload).await
                    }
                    _ => {
                        tracing::warn!(
                            "AwsAdapter::publish_with_type: Unknown adapter type '{}', falling back to default for topic: {}",
                            use_type,
                            topic_str
                        );
                        match default_type {
                            AwsAdapterType::Sqs => sqs.publish(topic_str, payload).await,
                            AwsAdapterType::EventBridge => eventbridge.publish(topic_str, payload).await,
                        }
                    }
                }
            }
        }
    }

    pub async fn subscribe_fn<F, Fut>(&self, topic: impl Into<String>, handler: F) -> common::Result<()>
    where
        F: Fn(common::Message) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = common::Result<()>> + Send + 'static,
    {
        self.subscribe_with_type(topic, handler, None).await
    }

    pub async fn subscribe_with_type<F, Fut>(
        &self,
        topic: impl Into<String>,
        handler: F,
        adapter_type: Option<&str>,
    ) -> common::Result<()>
    where
        F: Fn(common::Message) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = common::Result<()>> + Send + 'static,
    {
        let topic_str = topic.into();
        match self {
            AwsAdapter::Sqs(adapter) => adapter.subscribe_fn(topic_str, handler).await,
            AwsAdapter::EventBridge(adapter) => adapter.subscribe_fn(topic_str, handler).await,
            AwsAdapter::Both { sqs, eventbridge, default_type } => {
                let use_type = adapter_type
                    .map(|s| s.to_lowercase())
                    .unwrap_or_else(|| match default_type {
                        AwsAdapterType::Sqs => "sqs".to_string(),
                        AwsAdapterType::EventBridge => "eventbridge".to_string(),
                    });

                tracing::info!(
                    "AwsAdapter::subscribe_with_type: Both mode - requested: {:?}, using: {}, topic: {}",
                    adapter_type,
                    use_type,
                    topic_str
                );

                match use_type.as_str() {
                    "sqs" => {
                        tracing::info!("AwsAdapter::subscribe_with_type: Using SQS for subscription - topic: {}", topic_str);
                        sqs.subscribe_fn(topic_str, handler).await
                    }
                    "eventbridge" => {
                        tracing::info!("AwsAdapter::subscribe_with_type: Using EventBridge for subscription - topic: {}", topic_str);
                        eventbridge.subscribe_fn(topic_str, handler).await
                    }
                    _ => {
                        tracing::warn!(
                            "AwsAdapter::subscribe_with_type: Unknown adapter type '{}', falling back to default for topic: {}",
                            use_type,
                            topic_str
                        );
                        match default_type {
                            AwsAdapterType::Sqs => sqs.subscribe_fn(topic_str, handler).await,
                            AwsAdapterType::EventBridge => eventbridge.subscribe_fn(topic_str, handler).await,
                        }
                    }
                }
            }
        }
    }

    pub async fn list_topics(&self) -> Vec<String> {
        match self {
            AwsAdapter::Sqs(adapter) => adapter.list_topics().await,
            AwsAdapter::EventBridge(adapter) => adapter.list_topics().await,
            AwsAdapter::Both { sqs, eventbridge, default_type: _ } => {
                let mut topics = sqs.list_topics().await;
                let mut eb_topics = eventbridge.list_topics().await;
                topics.append(&mut eb_topics);
                topics.sort();
                topics.dedup();
                topics
            }
        }
    }
}

