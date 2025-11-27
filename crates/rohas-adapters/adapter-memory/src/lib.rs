use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, info};

pub type Result<T> = std::result::Result<T, AdapterError>;

#[derive(Error, Debug)]
pub enum AdapterError {
    #[error("Channel error: {0}")]
    ChannelError(String),

    #[error("Topic not found: {0}")]
    TopicNotFound(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Message envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub topic: String,
    pub payload: serde_json::Value,
    pub timestamp: String,
    pub metadata: HashMap<String, String>,
}

impl Message {
    pub fn new(topic: impl Into<String>, payload: serde_json::Value) -> Self {
        use std::time::SystemTime;
        Self {
            topic: topic.into(),
            payload,
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .to_string(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Message handler trait
#[async_trait]
pub trait MessageHandler: Send + Sync {
    async fn handle(&self, message: Message) -> Result<()>;
}

/// Memory-based message broker
pub struct MemoryAdapter {
    channels: Arc<RwLock<HashMap<String, broadcast::Sender<Message>>>>,
    buffer_size: usize,
}

impl MemoryAdapter {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            buffer_size,
        }
    }

    /// Create or get a channel for a topic
    async fn get_or_create_channel(&self, topic: &str) -> broadcast::Sender<Message> {
        let mut channels = self.channels.write().await;

        if let Some(sender) = channels.get(topic) {
            sender.clone()
        } else {
            let (sender, _) = broadcast::channel(self.buffer_size);
            channels.insert(topic.to_string(), sender.clone());
            info!("Created channel for topic: {}", topic);
            sender
        }
    }

    /// Publish a message to a topic
    pub async fn publish(
        &self,
        topic: impl Into<String>,
        payload: serde_json::Value,
    ) -> Result<()> {
        let topic = topic.into();
        let message = Message::new(topic.clone(), payload);

        let sender = self.get_or_create_channel(&topic).await;

        sender
            .send(message)
            .map_err(|e| AdapterError::ChannelError(format!("Failed to publish: {}", e)))?;

        debug!("Published message to topic: {}", topic);
        Ok(())
    }

    /// Subscribe to a topic with a handler
    pub async fn subscribe<H>(&self, topic: impl Into<String>, handler: Arc<H>) -> Result<()>
    where
        H: MessageHandler + 'static,
    {
        let topic = topic.into();
        let sender = self.get_or_create_channel(&topic).await;
        let mut receiver = sender.subscribe();

        info!("Subscribed to topic: {}", topic);

        tokio::spawn(async move {
            while let Ok(message) = receiver.recv().await {
                if let Err(e) = handler.handle(message).await {
                    tracing::error!("Handler error: {}", e);
                }
            }
        });

        Ok(())
    }

    /// Subscribe with a closure
    pub async fn subscribe_fn<F, Fut>(&self, topic: impl Into<String>, handler: F) -> Result<()>
    where
        F: Fn(Message) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        struct ClosureHandler<F, Fut>
        where
            F: Fn(Message) -> Fut + Send + Sync,
            Fut: std::future::Future<Output = Result<()>> + Send,
        {
            func: F,
        }

        #[async_trait]
        impl<F, Fut> MessageHandler for ClosureHandler<F, Fut>
        where
            F: Fn(Message) -> Fut + Send + Sync,
            Fut: std::future::Future<Output = Result<()>> + Send,
        {
            async fn handle(&self, message: Message) -> Result<()> {
                (self.func)(message).await
            }
        }

        let handler = Arc::new(ClosureHandler { func: handler });
        self.subscribe(topic, handler).await
    }

    /// Get list of all topics
    pub async fn list_topics(&self) -> Vec<String> {
        let channels = self.channels.read().await;
        channels.keys().cloned().collect()
    }

    /// Get subscriber count for a topic
    pub async fn subscriber_count(&self, topic: &str) -> usize {
        let channels = self.channels.read().await;
        channels
            .get(topic)
            .map(|sender| sender.receiver_count())
            .unwrap_or(0)
    }
}

impl Default for MemoryAdapter {
    fn default() -> Self {
        Self::new(1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_publish_subscribe() {
        let adapter = MemoryAdapter::new(10);

        let received = Arc::new(RwLock::new(Vec::new()));
        let received_clone = received.clone();

        adapter
            .subscribe_fn("test_topic", move |msg| {
                let received = received_clone.clone();
                async move {
                    received.write().await.push(msg.payload.clone());
                    Ok(())
                }
            })
            .await
            .unwrap();

        sleep(Duration::from_millis(10)).await;

        adapter
            .publish("test_topic", serde_json::json!({"value": 42}))
            .await
            .unwrap();

        sleep(Duration::from_millis(10)).await;

        let messages = received.read().await;
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["value"], 42);
    }
}
