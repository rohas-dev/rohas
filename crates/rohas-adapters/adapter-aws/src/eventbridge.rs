use crate::common::{AdapterError, Message, MessageHandler, Result};
use aws_sdk_eventbridge::Client as EventBridgeClient;
use aws_sdk_sqs::Client as SqsClient;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone)]
pub struct EventBridgeConfig {
    pub region: String,
    pub event_bus_name: Option<String>,
    pub source: Option<String>,
}

impl Default for EventBridgeConfig {
    fn default() -> Self {
        Self {
            region: "us-east-1".to_string(),
            event_bus_name: None, // Use default event bus
            source: Some("rohas".to_string()),
        }
    }
}

pub struct EventBridgeAdapter {
    client: EventBridgeClient,
    sqs_client: SqsClient,
    #[allow(dead_code)]
    config: EventBridgeConfig,
    event_bus_name: String,
    source: String,
    published_topics: Arc<RwLock<HashMap<String, ()>>>,
    queue_urls: Arc<RwLock<HashMap<String, String>>>, // topic -> queue_url
    rule_names: Arc<RwLock<HashMap<String, String>>>, // topic -> rule_name
}

impl EventBridgeAdapter {

    pub async fn new(config: EventBridgeConfig) -> Result<Self> {
        let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_sdk_eventbridge::config::Region::new(config.region.clone()))
            .load()
            .await;

        let client = EventBridgeClient::new(&aws_config);
        let sqs_client = SqsClient::new(&aws_config);

        let event_bus_name = config.event_bus_name.clone().unwrap_or_else(|| "default".to_string());
        let source = config.source.clone().unwrap_or_else(|| "rohas".to_string());

        info!(
            "Initialized EventBridge adapter for region: {}, event_bus: {}, source: {}",
            config.region, event_bus_name, source
        );

        Ok(Self {
            client,
            sqs_client,
            config,
            event_bus_name,
            source,
            published_topics: Arc::new(RwLock::new(HashMap::new())),
            queue_urls: Arc::new(RwLock::new(HashMap::new())),
            rule_names: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn publish(
        &self,
        topic: impl Into<String>,
        payload: serde_json::Value,
    ) -> Result<()> {
        let topic = topic.into();
        let message = Message::new(topic.clone(), payload.clone());

        {
            let mut topics = self.published_topics.write().await;
            topics.insert(topic.clone(), ());
        }

        let detail = serde_json::to_string(&message)
            .map_err(|e| AdapterError::Serialization(e))?;

        info!("Publishing EventBridge event - source: '{}', detail-type: '{}', detail length: {} bytes", 
            self.source, topic, detail.len());
        debug!("EventBridge event detail content: {}", detail);

        let mut event_builder = aws_sdk_eventbridge::types::PutEventsRequestEntry::builder()
            .source(&self.source)
            .detail_type(&topic)
            .detail(&detail);

        if self.event_bus_name != "default" {
            event_builder = event_builder.event_bus_name(&self.event_bus_name);
        }

        let event = event_builder.build();

        let send_result = self
            .client
            .put_events()
            .set_entries(Some(vec![event]))
            .send()
            .await;

        match send_result {
            Ok(response) => {
                let entries = response.entries();
                if !entries.is_empty() {
                    if let Some(entry) = entries.first() {
                        if let Some(error_code) = entry.error_code() {
                            error!(
                                "EventBridge publish failed for topic '{}': {} - {}",
                                topic,
                                error_code,
                                entry.error_message().unwrap_or("Unknown error")
                            );
                            return Err(AdapterError::AwsEventBridge(format!(
                                "Failed to publish event: {} - {}",
                                error_code,
                                entry.error_message().unwrap_or("Unknown error")
                            )));
                        }
                    }
                }
                info!(
                    "Published message to EventBridge topic: {} (event_bus: {}, source: {})",
                    topic, self.event_bus_name, self.source
                );
                Ok(())
            }
            Err(e) => {
                error!("Failed to send message to EventBridge '{}': {}", topic, e);
                Err(AdapterError::AwsEventBridge(format!(
                    "Failed to send event: {}",
                    e
                )))
            }
        }
    }

    async fn get_or_create_queue(&self, topic: &str) -> Result<String> {
        {
            let queue_urls = self.queue_urls.read().await;
            if let Some(url) = queue_urls.get(topic) {
                return Ok(url.clone());
            }
        }

        let queue_name = format!("rohas-eb-{}", topic)
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '-'
                }
            })
            .collect::<String>();

        info!("Checking if SQS queue '{}' exists...", queue_name);
        let get_queue_result = self
            .sqs_client
            .get_queue_url()
            .queue_name(&queue_name)
            .send()
            .await;

        let queue_url = match get_queue_result {
            Ok(response) => {
                if let Some(url) = response.queue_url() {
                    info!("Found existing SQS queue for EventBridge topic '{}': {}", topic, url);
                    url.to_string()
                } else {
                    error!("Queue URL not returned for '{}'", queue_name);
                    return Err(AdapterError::AwsEventBridge(format!(
                        "Queue URL not returned for '{}'",
                        queue_name
                    )));
                }
            }
            Err(e) => {
                warn!("SQS queue '{}' not found (error: {}), creating new queue...", queue_name, e);
                info!("Creating SQS queue for EventBridge topic '{}': {}", topic, queue_name);
                let create_result = self
                    .sqs_client
                    .create_queue()
                    .queue_name(&queue_name)
                    .send()
                    .await
                    .map_err(|e| {
                        error!("Failed to create SQS queue '{}': {}", queue_name, e);
                        AdapterError::AwsEventBridge(format!("Failed to create queue '{}': {}", queue_name, e))
                    })?;

                if let Some(url) = create_result.queue_url() {
                    info!("Created SQS queue for EventBridge topic '{}': {}", topic, url);
                    url.to_string()
                } else {
                    error!("Queue created but no URL returned for '{}'", queue_name);
                    return Err(AdapterError::AwsEventBridge(format!(
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

    async fn get_or_create_rule(&self, topic: &str, queue_arn: &str) -> Result<String> {
        {
            let rule_names = self.rule_names.read().await;
            if let Some(rule_name) = rule_names.get(topic) {
                return Ok(rule_name.clone());
            }
        }

        let rule_name = format!("rohas-rule-{}", topic)
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '-'
                }
            })
            .collect::<String>();

        let event_pattern = serde_json::json!({
            "source": [self.source],
            "detail-type": [topic]
        });
        
        info!("EventBridge rule pattern for topic '{}': {}", topic, event_pattern.to_string());
        info!("Expected event format: source='{}', detail-type='{}'", self.source, topic);

        let get_rule_result = self
            .client
            .describe_rule()
            .name(&rule_name)
            .set_event_bus_name(if self.event_bus_name != "default" {
                Some(self.event_bus_name.clone())
            } else {
                None
            })
            .send()
            .await;

        let target_id = format!("sqs-target-{}", topic);
        let target = aws_sdk_eventbridge::types::Target::builder()
            .id(&target_id)
            .arn(queue_arn)
            .build()
            .map_err(|e| {
                AdapterError::AwsEventBridge(format!("Failed to build target: {}", e))
            })?;

        match get_rule_result {
            Ok(rule_desc) => {
                info!("Found existing EventBridge rule for topic '{}': {}", topic, rule_name);
                if let Some(state) = rule_desc.state() {
                    match state {
                        aws_sdk_eventbridge::types::RuleState::Enabled => {
                            info!("EventBridge rule '{}' is ENABLED", rule_name);
                        }
                        aws_sdk_eventbridge::types::RuleState::Disabled => {
                            warn!("EventBridge rule '{}' is DISABLED - enabling it now...", rule_name);
                            match self
                                .client
                                .enable_rule()
                                .name(&rule_name)
                                .set_event_bus_name(if self.event_bus_name != "default" {
                                    Some(self.event_bus_name.clone())
                                } else {
                                    None
                                })
                                .send()
                                .await
                            {
                                Ok(_) => {
                                    info!("EventBridge rule '{}' has been enabled", rule_name);
                                }
                                Err(e) => {
                                    error!("Failed to enable EventBridge rule '{}': {}", rule_name, e);
                                    return Err(AdapterError::AwsEventBridge(format!(
                                        "Failed to enable rule '{}': {}",
                                        rule_name, e
                                    )));
                                }
                            }
                        }
                        _ => {
                            warn!("EventBridge rule '{}' has unknown state: {:?}", rule_name, state);
                        }
                    }
                }
                info!("Ensuring SQS queue target is added to existing rule '{}'", rule_name);
            }
            Err(e) => {
                warn!("EventBridge rule '{}' not found (error: {}), creating new rule...", rule_name, e);
                info!("Creating EventBridge rule for topic '{}': {}", topic, rule_name);
                
                let put_rule_result = self
                    .client
                    .put_rule()
                    .name(&rule_name)
                    .event_pattern(event_pattern.to_string())
                    .state(aws_sdk_eventbridge::types::RuleState::Enabled)
                    .set_event_bus_name(if self.event_bus_name != "default" {
                        Some(self.event_bus_name.clone())
                    } else {
                        None
                    })
                    .send()
                    .await
                    .map_err(|e| {
                        error!("Failed to create EventBridge rule '{}': {}", rule_name, e);
                        AdapterError::AwsEventBridge(format!("Failed to create rule '{}': {}", rule_name, e))
                    })?;

                info!("Created EventBridge rule '{}' (arn: {:?})", rule_name, put_rule_result.rule_arn());
            }
        }

        info!("Adding SQS queue as target to EventBridge rule '{}'", rule_name);
        info!("Target details: ID='{}', ARN='{}'", target_id, queue_arn);
        let put_targets_result = self
            .client
            .put_targets()
            .rule(&rule_name)
            .set_targets(Some(vec![target]))
            .set_event_bus_name(if self.event_bus_name != "default" {
                Some(self.event_bus_name.clone())
            } else {
                None
            })
            .send()
            .await
            .map_err(|e| {
                error!("Failed to add target to EventBridge rule '{}': {}", rule_name, e);
                AdapterError::AwsEventBridge(format!("Failed to add target to rule '{}': {}", rule_name, e))
            })?;

        let failed_entries = put_targets_result.failed_entries();
        if !failed_entries.is_empty() {
            error!("Failed to add target to EventBridge rule '{}': {:?}", rule_name, failed_entries);
            for entry in failed_entries {
                error!("  - Error Code: {}, Error Message: {}", 
                    entry.error_code().unwrap_or("unknown"),
                    entry.error_message().unwrap_or("unknown"));
            }
            return Err(AdapterError::AwsEventBridge(format!(
                "Failed to add target to rule '{}': {:?}",
                rule_name, failed_entries
            )));
        }

        info!("Successfully added SQS queue as target to EventBridge rule '{}'", rule_name);
        info!("Target configuration: Queue ARN='{}', Target ID='{}'", queue_arn, target_id);

        let list_targets_result = self
            .client
            .list_targets_by_rule()
            .rule(&rule_name)
            .set_event_bus_name(if self.event_bus_name != "default" {
                Some(self.event_bus_name.clone())
            } else {
                None
            })
            .send()
            .await;

        if let Ok(targets_response) = list_targets_result {
            let targets = targets_response.targets();
            if targets.is_empty() {
                error!("CRITICAL: EventBridge rule '{}' has NO TARGETS configured!", rule_name);
                return Err(AdapterError::AwsEventBridge(format!(
                    "Rule '{}' has no targets configured",
                    rule_name
                )));
            } else {
                info!("Verified: EventBridge rule '{}' has {} target(s) configured", rule_name, targets.len());
                for target in targets {
                    let target_arn = target.arn();
                    if target_arn == queue_arn {
                        info!("Target verified: SQS queue ARN '{}' is configured as target", queue_arn);
                    } else {
                        warn!("Found target with different ARN: {} (expected: {})", target_arn, queue_arn);
                    }
                }
            }
        } else {
            warn!("Could not list targets for rule '{}'", rule_name);
        }

        let verify_result = self
            .client
            .describe_rule()
            .name(&rule_name)
            .set_event_bus_name(if self.event_bus_name != "default" {
                Some(self.event_bus_name.clone())
            } else {
                None
            })
            .send()
            .await;

        if let Ok(rule_desc) = verify_result {
            if let Some(state) = rule_desc.state() {
                match state {
                    aws_sdk_eventbridge::types::RuleState::Enabled => {
                        info!("Verified: EventBridge rule '{}' is ENABLED and ready", rule_name);
                    }
                    aws_sdk_eventbridge::types::RuleState::Disabled => {
                        error!("CRITICAL: EventBridge rule '{}' is still DISABLED after setup!", rule_name);
                        return Err(AdapterError::AwsEventBridge(format!(
                            "Rule '{}' is disabled and could not be enabled",
                            rule_name
                        )));
                    }
                    _ => {
                        warn!("EventBridge rule '{}' has unknown state: {:?}", rule_name, state);
                    }
                }
            }
        } else {
            warn!("Could not verify rule state after setup");
        }

        {
            let mut rule_names = self.rule_names.write().await;
            rule_names.insert(topic.to_string(), rule_name.clone());
        }

        Ok(rule_name)
    }

    async fn get_queue_arn(&self, queue_url: &str) -> Result<String> {
        let queue_name = queue_url
            .split('/')
            .last()
            .ok_or_else(|| {
                error!("Invalid queue URL format: {}", queue_url);
                AdapterError::AwsEventBridge(format!("Invalid queue URL: {}", queue_url))
            })?;

        info!("Retrieving ARN for SQS queue '{}'...", queue_name);
        
        let attributes_result = self
            .sqs_client
            .get_queue_attributes()
            .queue_url(queue_url)
            .attribute_names(aws_sdk_sqs::types::QueueAttributeName::QueueArn)
            .send()
            .await
            .map_err(|e| {
                error!("Failed to get queue attributes for '{}': {}", queue_name, e);
                AdapterError::AwsEventBridge(format!("Failed to get queue attributes for '{}': {}", queue_name, e))
            })?;

        if let Some(attributes) = attributes_result.attributes() {
            if let Some(arn) = attributes.get(&aws_sdk_sqs::types::QueueAttributeName::QueueArn) {
                info!("Retrieved ARN for queue '{}': {}", queue_name, arn);
                return Ok(arn.clone());
            }
        }

        error!("Queue ARN not found in attributes for queue '{}'", queue_name);
        Err(AdapterError::AwsEventBridge(format!(
            "Queue ARN not found for queue '{}'",
            queue_name
        )))
    }

    pub async fn subscribe_fn<F, Fut>(&self, topic: impl Into<String>, handler: F) -> Result<()>
    where
        F: Fn(Message) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        let topic = topic.into();
        info!("=== Setting up EventBridge subscription for topic: {} ===", topic);

        info!("Step 1: Creating/getting SQS queue for topic '{}'...", topic);
        let queue_url = match self.get_or_create_queue(&topic).await {
            Ok(url) => {
                info!("SQS queue ready for EventBridge topic '{}': {}", topic, url);
                url
            }
            Err(e) => {
                error!("Failed to create/get SQS queue for topic '{}': {}", topic, e);
                return Err(e);
            }
        };

        info!("Step 2: Getting SQS queue ARN for topic '{}'...", topic);
        let queue_arn = match self.get_queue_arn(&queue_url).await {
            Ok(arn) => {
                info!("SQS queue ARN for topic '{}': {}", topic, arn);
                arn
            }
            Err(e) => {
                error!("Failed to get SQS queue ARN for topic '{}': {}", topic, e);
                return Err(e);
            }
        };

        info!("Step 3: Creating/getting EventBridge rule for topic '{}'...", topic);
        let rule_name = match self.get_or_create_rule(&topic, &queue_arn).await {
            Ok(name) => {
                info!("EventBridge rule ready for topic '{}': {}", topic, name);
                name
            }
            Err(e) => {
                error!("Failed to create/get EventBridge rule for topic '{}': {}", topic, e);
                return Err(e);
            }
        };

        let account_id = queue_arn
            .split(':')
            .nth(4)
            .unwrap_or("*");
        
        // EventBridge rule ARN format:
        // - Default bus: arn:aws:events:region:account-id:rule/rule-name
        // - Custom bus: arn:aws:events:region:account-id:rule/event-bus-name/rule-name
        let rule_arn = if self.event_bus_name == "default" {
            format!(
                "arn:aws:events:{}:{}:rule/{}",
                self.config.region,
                account_id,
                rule_name
            )
        } else {
            format!(
                "arn:aws:events:{}:{}:rule/{}/{}",
                self.config.region,
                account_id,
                self.event_bus_name,
                rule_name
            )
        };
        
        info!("Step 4: Setting up SQS queue policy for EventBridge access (rule ARN: {})...", rule_arn);
        
        let policy = serde_json::json!({
            "Version": "2012-10-17",
            "Statement": [{
                "Effect": "Allow",
                "Principal": {
                    "Service": "events.amazonaws.com"
                },
                "Action": "sqs:SendMessage",
                "Resource": queue_arn,
                "Condition": {
                    "ArnEquals": {
                        "aws:SourceArn": rule_arn
                    }
                }
            }]
        });

        match self
            .sqs_client
            .set_queue_attributes()
            .queue_url(&queue_url)
            .attributes(
                aws_sdk_sqs::types::QueueAttributeName::Policy,
                policy.to_string(),
            )
            .send()
            .await
        {
            Ok(_) => {
                info!("Successfully set SQS queue policy for EventBridge access");
            }
            Err(e) => {
                warn!("Failed to set SQS queue policy (this may be okay if policy already exists): {}", e);
            }
        }

        info!("Step 5: Starting SQS queue polling for topic '{}'...", topic);
        let sqs_client = self.sqs_client.clone();
        let topic_clone = topic.clone();
        let queue_url_clone = queue_url.clone();
        let queue_arn_clone = queue_arn.clone();
        let rule_name_clone = rule_name.clone();

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

        tokio::spawn(async move {
            info!("EventBridge subscription polling loop started for topic '{}' (queue: {})", topic_clone, queue_url);
            let mut poll_count = 0u64;
            loop {
                poll_count += 1;
                if poll_count % 10 == 0 {
                    info!("EventBridge polling loop still active for topic '{}' (poll #{}), queue: {}", topic_clone, poll_count, queue_url);
                }
                if poll_count == 1 || poll_count % 5 == 0 {
                    info!("Polling SQS queue for EventBridge topic '{}' (poll #{})...", topic_clone, poll_count);
                } else {
                    debug!("Polling SQS queue for EventBridge topic '{}' (poll #{})...", topic_clone, poll_count);
                }
                let receive_result = sqs_client
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
                            info!("Received {} message(s) from EventBridge queue for topic '{}'", messages.len(), topic_clone);
                            for sqs_message in messages {
                                if let Some(body) = sqs_message.body() {
                                    info!("Raw SQS message body for topic '{}': {}", topic_clone, body);
                                    
                                    debug!("Attempting to parse EventBridge message for topic '{}'", topic_clone);
                                    let message_result = {
                                        debug!("Trying to parse as array of events...");
                                        if let Ok(events_array) = serde_json::from_str::<Vec<serde_json::Value>>(body) {
                                            debug!("Successfully parsed as array with {} event(s)", events_array.len());
                                            if let Some(event) = events_array.first() {
                                                debug!("First event structure: {:?}", event);
                                                if let Some(detail_str) = event.get("detail").and_then(|d| d.as_str()) {
                                                    debug!("Found 'detail' field as string (length: {}): {}", detail_str.len(), detail_str);
                                                    match serde_json::from_str::<Message>(detail_str) {
                                                        Ok(msg) => {
                                                            debug!("Successfully parsed Message from detail string");
                                                            Some(msg)
                                                        }
                                                        Err(e) => {
                                                            debug!("Failed to parse Message from detail string: {}", e);
                                                            None
                                                        }
                                                    }
                                                } else if let Some(detail_obj) = event.get("detail") {
                                                    debug!("Found 'detail' field as object: {:?}", detail_obj);
                                                    match serde_json::from_value::<Message>(detail_obj.clone()) {
                                                        Ok(msg) => {
                                                            debug!("Successfully parsed Message from detail object");
                                                            Some(msg)
                                                        }
                                                        Err(e) => {
                                                            debug!("Failed to parse Message from detail object: {}", e);
                                                            None
                                                        }
                                                    }
                                                } else {
                                                    debug!("No 'detail' field found in event object");
                                                    None
                                                }
                                            } else {
                                                debug!("Array is empty");
                                                None
                                            }
                                        } else {
                                            debug!("Not an array, trying as single event object...");
                                            None
                                        }
                                    }.or_else(|| {
                                        debug!("Trying to parse as single event object...");
                                        if let Ok(event_obj) = serde_json::from_str::<serde_json::Value>(body) {
                                            debug!("Successfully parsed as event object");
                                            if let Some(detail_str) = event_obj.get("detail").and_then(|d| d.as_str()) {
                                                debug!("Found 'detail' field as string (length: {}): {}", detail_str.len(), detail_str);
                                                match serde_json::from_str::<Message>(detail_str) {
                                                    Ok(msg) => {
                                                        debug!("Successfully parsed Message from detail string");
                                                        Some(msg)
                                                    }
                                                    Err(e) => {
                                                        debug!("Failed to parse Message from detail string: {}", e);
                                                        None
                                                    }
                                                }
                                            } else if let Some(detail_obj) = event_obj.get("detail") {
                                                debug!("Found 'detail' field as object: {:?}", detail_obj);
                                                match serde_json::from_value::<Message>(detail_obj.clone()) {
                                                    Ok(msg) => {
                                                        debug!("Successfully parsed Message from detail object");
                                                        Some(msg)
                                                    }
                                                    Err(e) => {
                                                        debug!("Failed to parse Message from detail object: {}", e);
                                                        None
                                                    }
                                                }
                                            } else {
                                                debug!("No 'detail' field found in event object");
                                                None
                                            }
                                        } else {
                                            debug!("Not a valid JSON object, trying direct Message parse...");
                                            None
                                        }
                                    }).or_else(|| {
                                        debug!("Trying to parse body directly as Message...");
                                        match serde_json::from_str::<Message>(body) {
                                            Ok(msg) => {
                                                debug!("Successfully parsed body directly as Message");
                                                Some(msg)
                                            }
                                            Err(e) => {
                                                debug!("Failed to parse body directly as Message: {}", e);
                                                None
                                            }
                                        }
                                    });
                                    
                                    let message_result = message_result.ok_or_else(|| {
                                        let last_error = serde_json::from_str::<Message>(body)
                                            .map_err(|e| e)
                                            .unwrap_err();
                                        debug!("All parsing attempts failed. Last error: {}", last_error);
                                        last_error
                                    });

                                    match message_result {
                                        Ok(message) => {
                                            info!("Successfully parsed EventBridge message for topic '{}'", topic_clone);
                                            info!("Message topic: {}, payload: {:?}", message.topic, message.payload);
                                            info!("Calling handler for EventBridge message...");
                                            if let Err(e) = handler.handle(message).await {
                                                error!("Handler error for EventBridge topic '{}': {}", topic_clone, e);
                                            } else {
                                                info!("Handler completed successfully for EventBridge topic '{}'", topic_clone);
                                            }

                                            if let Some(receipt_handle) = sqs_message.receipt_handle() {
                                                if let Err(e) = sqs_client
                                                    .delete_message()
                                                    .queue_url(&queue_url)
                                                    .receipt_handle(receipt_handle)
                                                    .send()
                                                    .await
                                                {
                                                    warn!(
                                                        "Failed to delete message from EventBridge queue '{}': {}",
                                                        queue_url, e
                                                    );
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            error!(
                                                "Failed to deserialize EventBridge message for topic '{}': {}. Body: {}",
                                                topic_clone, e, body
                                            );
                                            if let Some(receipt_handle) = sqs_message.receipt_handle() {
                                                let _ = sqs_client
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
                            debug!("No messages received from EventBridge queue for topic '{}' (this is normal, continuing to poll...)", topic_clone);
                        }
                    }
                    Err(e) => {
                        error!(
                            "Error receiving messages from EventBridge queue '{}' for topic '{}': {}. Retrying in 5 seconds...",
                            queue_url, topic_clone, e
                        );
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                }
            }
        });

        info!("=== EventBridge subscription set up successfully for topic: {} ===", topic);
        info!("Summary:");
        info!("  - SQS Queue URL: {}", queue_url_clone);
        info!("  - SQS Queue ARN: {}", queue_arn_clone);
        info!("  - EventBridge Rule: {}", rule_name_clone);
        info!("  - Event Pattern: source='{}', detail-type='{}'", self.source, topic);
        info!("  - Event Bus: {}", self.event_bus_name);
        info!("  - Target: SQS queue '{}' (ARN: {})", queue_url_clone, queue_arn_clone);
        info!("  - Polling: Active (long polling enabled, 20s wait time)");
        info!("  - Next: Events matching the pattern will be routed to the SQS queue");
        let rule_arn_final = if self.event_bus_name == "default" {
            format!(
                "arn:aws:events:{}:{}:rule/{}",
                self.config.region,
                queue_arn_clone.split(':').nth(4).unwrap_or("unknown"),
                rule_name_clone
            )
        } else {
            format!(
                "arn:aws:events:{}:{}:rule/{}/{}",
                self.config.region,
                queue_arn_clone.split(':').nth(4).unwrap_or("unknown"),
                self.event_bus_name,
                rule_name_clone
            )
        };
        info!("  - Rule ARN: {}", rule_arn_final);
        info!("  - To verify: Check AWS EventBridge console for rule '{}' and ensure it has the SQS queue as a target", rule_name_clone);
        Ok(())
    }

    pub async fn list_topics(&self) -> Vec<String> {
        let topics = self.published_topics.read().await;
        topics.keys().cloned().collect()
    }
}

