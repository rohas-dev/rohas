use crate::common::{AdapterError, Message, MessageHandler, Result};
use aws_sdk_sqs::{
    types::{MessageAttributeValue, QueueAttributeName},
    Client as SqsClient,
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone)]
pub struct SqsConfig {
    pub region: String,
    pub queue_prefix: Option<String>,
    pub visibility_timeout_seconds: Option<i32>,
    pub message_retention_seconds: Option<i32>,
    pub receive_wait_time_seconds: Option<i32>, // Long polling wait time
}

impl Default for SqsConfig {
    fn default() -> Self {
        Self {
            region: "us-east-1".to_string(),
            queue_prefix: Some("rohas-".to_string()),
            visibility_timeout_seconds: Some(30),
            message_retention_seconds: Some(345600), // 4 days
            receive_wait_time_seconds: Some(20),      // Long polling
        }
    }
}

pub struct SqsAdapter {
    client: SqsClient,
    config: SqsConfig,
    queue_urls: Arc<RwLock<HashMap<String, String>>>, // topic -> queue_url
}

impl SqsAdapter {
    pub async fn new(config: SqsConfig) -> Result<Self> {
        let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_sdk_sqs::config::Region::new(config.region.clone()))
            .load()
            .await;

        let client = SqsClient::new(&aws_config);

        info!(
            "Initialized SQS adapter for region: {}",
            config.region
        );

        Ok(Self {
            client,
            config,
            queue_urls: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    async fn get_or_create_queue(&self, topic: &str) -> Result<String> {
        {
            let queue_urls = self.queue_urls.read().await;
            if let Some(url) = queue_urls.get(topic) {
                return Ok(url.clone());
            }
        }

        let queue_name = if let Some(prefix) = &self.config.queue_prefix {
            format!("{}{}", prefix, topic)
        } else {
            topic.to_string()
        };

        let queue_name = queue_name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '-'
                }
            })
            .collect::<String>();

        let get_queue_result = self
            .client
            .get_queue_url()
            .queue_name(&queue_name)
            .send()
            .await;

        let queue_url = match get_queue_result {
            Ok(response) => {
                if let Some(url) = response.queue_url() {
                    info!("Found existing queue for topic '{}': {}", topic, url);
                    url.to_string()
                } else {
                    return Err(AdapterError::QueueNotFound(queue_name));
                }
            }
            Err(_) => {
                debug!("Queue '{}' not found, creating...", queue_name);

                let mut create_request = self.client.create_queue().queue_name(&queue_name);

                let mut attributes = HashMap::new();
                if let Some(visibility) = self.config.visibility_timeout_seconds {
                    attributes.insert(
                        QueueAttributeName::VisibilityTimeout,
                        visibility.to_string(),
                    );
                }
                if let Some(retention) = self.config.message_retention_seconds {
                    attributes.insert(
                        QueueAttributeName::MessageRetentionPeriod,
                        retention.to_string(),
                    );
                }
                if let Some(wait_time) = self.config.receive_wait_time_seconds {
                    attributes.insert(
                        QueueAttributeName::ReceiveMessageWaitTimeSeconds,
                        wait_time.to_string(),
                    );
                }

                if !attributes.is_empty() {
                    create_request = create_request.set_attributes(Some(attributes));
                }

                let create_result = create_request.send().await.map_err(|e| {
                    AdapterError::AwsSqs(format!("Failed to create queue '{}': {}", queue_name, e))
                })?;

                if let Some(url) = create_result.queue_url() {
                    info!("Created queue for topic '{}': {}", topic, url);
                    url.to_string()
                } else {
                    return Err(AdapterError::AwsSqs(format!(
                        "Queue created but no URL returned for '{}'",
                        queue_name
                    )));
                }
            }
        };

        {
            let mut queue_urls = self.queue_urls.write().await;
            queue_urls.insert(topic.to_string(), queue_url.clone());
        }

        Ok(queue_url)
    }

    pub async fn publish(
        &self,
        topic: impl Into<String>,
        payload: serde_json::Value,
    ) -> Result<()> {
        let topic = topic.into();
        tracing::info!("SqsAdapter::publish: Starting publish for topic: {}", topic);
        
        let message = Message::new(topic.clone(), payload);

        let message_body = serde_json::to_string(&message)
            .map_err(|e| {
                tracing::error!("SqsAdapter::publish: Serialization error for topic {}: {}", topic, e);
                AdapterError::Serialization(e)
            })?;

        tracing::debug!("SqsAdapter::publish: Message serialized, getting/creating queue for topic: {}", topic);
        
        let queue_url = self.get_or_create_queue(&topic).await.map_err(|e| {
            tracing::error!("SqsAdapter::publish: Failed to get/create queue for topic {}: {}", topic, e);
            e
        })?;
        
        tracing::info!("SqsAdapter::publish: Queue URL obtained: {} for topic: {}", queue_url, topic);

        let mut attributes = HashMap::new();
        attributes.insert(
            "topic".to_string(),
            MessageAttributeValue::builder()
                .data_type("String")
                .string_value(&topic)
                .build()
                .map_err(|e| AdapterError::AwsSqs(format!("Failed to build attribute: {}", e)))?,
        );
        attributes.insert(
            "timestamp".to_string(),
            MessageAttributeValue::builder()
                .data_type("String")
                .string_value(&message.timestamp)
                .build()
                .map_err(|e| AdapterError::AwsSqs(format!("Failed to build attribute: {}", e)))?,
        );

        let send_result = self
            .client
            .send_message()
            .queue_url(&queue_url)
            .message_body(&message_body)
            .set_message_attributes(Some(attributes))
            .send()
            .await;

        match send_result {
            Ok(response) => {
                if let Some(message_id) = response.message_id() {
                    info!("Published message to SQS topic: {} (queue: {}, message_id: {})", topic, queue_url, message_id);
                } else {
                    info!("Published message to SQS topic: {} (queue: {})", topic, queue_url);
                }
                Ok(())
            }
            Err(e) => {
                error!("Failed to send message to SQS queue '{}' for topic '{}': {}", queue_url, topic, e);
                let error_msg = format!(
                    "Failed to send message to queue '{}': {}",
                    queue_url, e
                );
                tracing::error!("SqsAdapter::publish: Error details - {}", error_msg);
                Err(AdapterError::AwsSqs(error_msg))
            }
        }
    }

    pub async fn subscribe<H>(&self, topic: impl Into<String>, handler: Arc<H>) -> Result<()>
    where
        H: MessageHandler + 'static,
    {
        let topic = topic.into();
        let queue_url = self.get_or_create_queue(&topic).await?;

        info!("Subscribing to topic: {} (queue: {})", topic, queue_url);

        let client = self.client.clone();
        let handler = handler.clone();
        let topic_clone = topic.clone();

        tokio::spawn(async move {
            info!("SQS subscription polling loop started for topic '{}' (queue: {})", topic_clone, queue_url);
            let mut poll_count = 0u64;
            loop {
                poll_count += 1;
                if poll_count % 5 == 0 {
                    info!("SQS polling loop still active for topic '{}' (poll #{}), queue: {}", topic_clone, poll_count, queue_url);
                } else if poll_count <= 3 {
                    info!("SQS polling loop active for topic '{}' (poll #{}), queue: {}", topic_clone, poll_count, queue_url);
                } else {
                    debug!("Polling SQS queue for topic '{}' (poll #{})...", topic_clone, poll_count);
                }
                let receive_result = client
                    .receive_message()
                    .queue_url(&queue_url)
                    .max_number_of_messages(10)
                    .wait_time_seconds(20)
                    .send()
                    .await;

                match receive_result {
                    Ok(response) => {
                        let messages = response.messages();
                        if !messages.is_empty() {
                            info!("Received {} message(s) from SQS queue for topic '{}'", messages.len(), topic_clone);
                            for sqs_message in messages {
                                if let Some(body) = sqs_message.body() {
                                    info!("Raw SQS message body for topic '{}': {}", topic_clone, body);
                                    match serde_json::from_str::<Message>(body) {
                                        Ok(message) => {
                                            info!("Successfully parsed SQS message for topic '{}'", topic_clone);
                                            info!("Message topic: {}, payload: {:?}", message.topic, message.payload);
                                            info!("Calling handler for SQS message...");
                                            if let Err(e) = handler.handle(message).await {
                                                error!("Handler error for SQS topic '{}': {}", topic_clone, e);
                                            } else {
                                                info!("Handler completed successfully for SQS topic '{}'", topic_clone);
                                            }

                                            if let Some(receipt_handle) = sqs_message.receipt_handle() {
                                                if let Err(e) = client
                                                    .delete_message()
                                                    .queue_url(&queue_url)
                                                    .receipt_handle(receipt_handle)
                                                    .send()
                                                    .await
                                                {
                                                    warn!(
                                                        "Failed to delete message from queue '{}': {}",
                                                        queue_url, e
                                                    );
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            error!(
                                                "Failed to deserialize SQS message for topic '{}': {}. Body: {}",
                                                topic_clone, e, body
                                            );
                                            if let Some(receipt_handle) = sqs_message.receipt_handle() {
                                                let _ = client
                                                    .delete_message()
                                                    .queue_url(&queue_url)
                                                    .receipt_handle(receipt_handle)
                                                    .send()
                                                    .await;
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            debug!("No messages received from SQS queue for topic '{}' (this is normal, continuing to poll...)", topic_clone);
                        }
                    }
                    Err(e) => {
                        error!(
                            "Error receiving messages from SQS queue '{}' for topic '{}': {}. Retrying in 5 seconds...",
                            queue_url, topic_clone, e
                        );
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                }
            }
        });

        Ok(())
    }

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

    pub async fn list_topics(&self) -> Vec<String> {
        let queue_urls = self.queue_urls.read().await;
        queue_urls.keys().cloned().collect()
    }
}

