use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use rohas_codegen::templates;
use serde_json::{json, Value};
use uuid::Uuid;
use chrono::Utc;

use crate::{config, api::ApiState};

pub async fn websocket_handler(
    socket: WebSocket,
    state: ApiState,
    ws_name: String,
) {
    let connection_id = Uuid::new_v4().to_string();
    let ws_config = state
        .schema
        .websockets
        .iter()
        .find(|ws| ws.name == ws_name)
        .expect("WebSocket config not found");

    let (mut sender, mut receiver) = socket.split();
    let connected_at = Utc::now();
 
    let connection = json!({
        "connection_id": connection_id,
        "path": ws_config.path,
        "connected_at": connected_at.to_rfc3339(),
    });
 
    if !ws_config.on_connect.is_empty() {
        for handler_name in &ws_config.on_connect {
            let handler_name = match state.config.language {
                config::Language::TypeScript => handler_name.clone(),
                config::Language::Python => templates::to_snake_case(handler_name.as_str()),
            };

            let payload = connection.clone();
            let mut context = rohas_runtime::HandlerContext::new(&handler_name, payload);
            context.metadata.insert("websocket_name".to_string(), ws_name.clone());
            let result = state
                .executor
                .execute_with_context(context)
                .await;

            if let Ok(result) = result {
                if result.success {
                    if let Some(data) = result.data {
                      
                        if let Ok(msg) = serde_json::to_string(&data) {
                            tracing::debug!("Sending welcome message: {}", msg);
                            if let Err(e) = sender.send(Message::Text(msg.into())).await {
                                tracing::error!("Failed to send welcome message: {}", e);
                            }
                        } else {
                            tracing::warn!("Failed to serialize welcome message");
                        }
                    } else {
                        tracing::debug!("Handler returned no data (None)");
                    }
                } else {
                    tracing::warn!("Handler execution failed: {:?}", result.error);
                }
            } else {
                tracing::error!("Handler execution error: {:?}", result);
            }
        }
    }

 
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                let text_str = text.to_string();
                let message_data: Value = serde_json::from_str(&text_str)
                    .unwrap_or_else(|_| json!({ "data": text_str }));

                let message = json!({
                    "data": message_data,
                    "timestamp": Utc::now().to_rfc3339(),
                });
 
                if !ws_config.on_message.is_empty() {
                    for handler_name in &ws_config.on_message {
                        let handler_name = match state.config.language {
                            config::Language::TypeScript => handler_name.clone(),
                            config::Language::Python => templates::to_snake_case(handler_name.as_str()),
                        };

                        let handler_payload = json!({
                            "message": message,
                            "connection": connection,
                        });

                        let mut context = rohas_runtime::HandlerContext::new(&handler_name, handler_payload);
                        context.metadata.insert("websocket_name".to_string(), ws_name.clone());
                        let result = state
                            .executor
                            .execute_with_context(context)
                            .await;

                        if let Ok(result) = result {
                            if result.success {
                            
                                if let Some(data) = result.data {
                                    if let Ok(msg) = serde_json::to_string(&data) {
                                        tracing::debug!("Sending response message: {}", msg);
                                        if let Err(e) = sender.send(Message::Text(msg.into())).await {
                                            tracing::error!("Failed to send response message: {}", e);
                                        }
                                    } else {
                                        tracing::warn!("Failed to serialize response message");
                                    }
                                } else {
                                    tracing::debug!("Handler returned no data (None)");
                                }
                            } else {
                                tracing::warn!("Handler execution failed: {:?}", result.error);
                            }
 
                            for triggered_event in &result.triggers {
                                if let Err(e) = state
                                    .event_bus
                                    .emit(&triggered_event.event_name, triggered_event.payload.clone())
                                    .await
                                {
                                    tracing::error!(
                                        "Failed to emit event {} from websocket {}: {}",
                                        triggered_event.event_name,
                                        ws_name,
                                        e
                                    );
                                }
                            }

                            for trigger in &ws_config.triggers {
                            
                                if let Some(payload) = result.auto_trigger_payloads.get(trigger) {
                                    if let Err(e) = state.event_bus.emit(trigger, payload.clone()).await {
                                        tracing::error!(
                                            "Failed to emit auto-triggered event {} from websocket {}: {}",
                                            trigger,
                                            ws_name,
                                            e
                                        );
                                    }
                                } else {
                                    tracing::debug!(
                                        "Skipping auto-trigger {} from websocket {}: no payload set by handler",
                                        trigger,
                                        ws_name
                                    );
                                }
                            }
                        } else {
                            tracing::error!("Handler execution error: {:?}", result);
                        }
                    }
                }
            }
            Ok(Message::Close(_)) => {
                break;
            }
            Err(e) => {
                tracing::error!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

 
    if !ws_config.on_disconnect.is_empty() {
        for handler_name in &ws_config.on_disconnect {
            let handler_name = match state.config.language {
                config::Language::TypeScript => handler_name.clone(),
                config::Language::Python => templates::to_snake_case(handler_name.as_str()),
            };

            let payload = connection.clone();
            let mut context = rohas_runtime::HandlerContext::new(&handler_name, payload);
            context.metadata.insert("websocket_name".to_string(), ws_name.clone());
            let _ = state.executor.execute_with_context(context).await;
        }
    }
}

