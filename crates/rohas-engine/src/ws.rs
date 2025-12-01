use axum::extract::ws::{Message, WebSocket};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use rohas_codegen::templates;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Instant;
use tracing::{debug, error, warn};
use uuid::Uuid;

use crate::{api::ApiState, config, trace::TraceEntryType};

async fn execute_websocket_middlewares(
    state: ApiState,
    middlewares: &[String],
    payload: serde_json::Value,
    trace_id: &str,
    ws_name: &str,
) -> Result<(), String> {
    if middlewares.is_empty() {
        return Ok(());
    }

    debug!("Executing {} middlewares for WebSocket: {}", middlewares.len(), ws_name);

    for middleware_name in middlewares {
        let middleware_handler_name = match state.config.language {
            config::Language::TypeScript => middleware_name.clone(),
            config::Language::Python => templates::to_snake_case(middleware_name.as_str()),
        };

        debug!("Executing WebSocket middleware: {}", middleware_handler_name);

        let mut context = rohas_runtime::HandlerContext::new(&middleware_handler_name, payload.clone());
        context.metadata.insert("middleware".to_string(), "true".to_string());
        context.metadata.insert("websocket_name".to_string(), ws_name.to_string());

        let start = std::time::Instant::now();
        let result = state.executor.execute_with_context(context).await;
        let duration_ms = start.elapsed().as_millis() as u64;

        if let Ok(ref exec_result) = result {
            state
                .trace_store
                .add_step(
                    trace_id,
                    format!("middleware:{}", middleware_handler_name),
                    duration_ms.max(exec_result.execution_time_ms),
                    exec_result.success,
                    exec_result.error.clone(),
                )
                .await;
        }

        match result {
            Ok(exec_result) => {
                if !exec_result.success {
                    let error_msg = exec_result.error.unwrap_or_else(|| {
                        format!("Middleware '{}' rejected the WebSocket connection", middleware_name)
                    });
                    return Err(error_msg);
                }
            }
            Err(e) => {
                let error_msg = format!("Middleware '{}' execution failed: {}", middleware_name, e);
                return Err(error_msg);
            }
        }
    }

    Ok(())
}

pub async fn websocket_handler(socket: WebSocket, state: ApiState, ws_name: String) {
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

    // Start trace for connection
    let mut metadata = HashMap::new();
    metadata.insert("path".to_string(), ws_config.path.clone());
    metadata.insert("connection_id".to_string(), connection_id.clone());
    let connection_trace_id = state
        .trace_store
        .start_trace(
            format!("{} (connect)", ws_name),
            TraceEntryType::WebSocket,
            metadata,
        )
        .await;

    if !ws_config.middlewares.is_empty() {
        let middleware_payload = json!({
            "connection": connection.clone(),
            "websocket_name": ws_name,
        });
        
        let middleware_result = execute_websocket_middlewares(
            state.clone(),
            &ws_config.middlewares,
            middleware_payload,
            &connection_trace_id,
            &ws_name,
        )
        .await;

        if let Err(e) = middleware_result {
            error!("WebSocket middleware rejected connection: {}", e);
            state
                .trace_store
                .complete_trace(&connection_trace_id, crate::trace::TraceStatus::Failed, Some(e))
                .await;
            return;
        }
    }

    if !ws_config.on_connect.is_empty() {
        for handler_name in &ws_config.on_connect {
            let handler_name = match state.config.language {
                config::Language::TypeScript => handler_name.clone(),
                config::Language::Python => templates::to_snake_case(handler_name.as_str()),
            };

            let payload = connection.clone();
            let mut context = rohas_runtime::HandlerContext::new(&handler_name, payload);
            context
                .metadata
                .insert("websocket_name".to_string(), ws_name.clone());
            
            let start = Instant::now();
            let result = state.executor.execute_with_context(context).await;
            let duration_ms = start.elapsed().as_millis() as u64;

            // Add trace step
            if let Ok(ref exec_result) = result {
                state
                    .trace_store
                    .add_step(
                        &connection_trace_id,
                        handler_name.clone(),
                        duration_ms.max(exec_result.execution_time_ms),
                        exec_result.success,
                        exec_result.error.clone(),
                    )
                    .await;
            }

            if let Ok(result) = result {
                if result.success {
                    if let Some(data) = result.data {
                        if let Ok(msg) = serde_json::to_string(&data) {
                            debug!("Sending welcome message: {}", msg);
                            if let Err(e) = sender.send(Message::Text(msg.into())).await {
                                error!("Failed to send welcome message: {}", e);
                            }
                        } else {
                            warn!("Failed to serialize welcome message");
                        }
                    } else {
                        debug!("Handler returned no data (None)");
                    }
                } else {
                    warn!("Handler execution failed: {:?}", result.error);
                }
            } else {
                error!("Handler execution error: {:?}", result);
            }
        }
    }

    state
        .trace_store
        .complete_trace(&connection_trace_id, crate::trace::TraceStatus::Success, None)
        .await;

    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                let text_str = text.to_string();
                let message_data: Value =
                    serde_json::from_str(&text_str).unwrap_or_else(|_| json!({ "data": text_str }));

                let message = json!({
                    "data": message_data,
                    "timestamp": Utc::now().to_rfc3339(),
                });

                let mut message_metadata = HashMap::new();
                message_metadata.insert("path".to_string(), ws_config.path.clone());
                message_metadata.insert("connection_id".to_string(), connection_id.clone());
                let message_trace_id = state
                    .trace_store
                    .start_trace(
                        format!("{} (message)", ws_name),
                        TraceEntryType::WebSocket,
                        message_metadata,
                    )
                    .await;

                if !ws_config.on_message.is_empty() {
                    for handler_name in &ws_config.on_message {
                        let handler_name = match state.config.language {
                            config::Language::TypeScript => handler_name.clone(),
                            config::Language::Python => {
                                templates::to_snake_case(handler_name.as_str())
                            }
                        };

                        let handler_payload = json!({
                            "message": message,
                            "connection": connection,
                        });

                        let mut context =
                            rohas_runtime::HandlerContext::new(&handler_name, handler_payload);
                        context
                            .metadata
                            .insert("websocket_name".to_string(), ws_name.clone());
                        
                        let start = Instant::now();
                        let result = state.executor.execute_with_context(context).await;
                        let duration_ms = start.elapsed().as_millis() as u64;

                        // Collect triggered events with timestamps and duration, add trace step
                        let mut triggered_events = Vec::new();
                        if let Ok(ref exec_result) = result {
                            if exec_result.success {
                                // Add events from handler result triggers
                                for triggered_event in &exec_result.triggers {
                                    let trigger_start = std::time::Instant::now();
                                    // Emit the event and measure duration
                                    let emit_result = state
                                        .event_bus
                                        .emit(
                                            &triggered_event.event_name,
                                            triggered_event.payload.clone(),
                                        )
                                        .await;
                                    let trigger_duration = trigger_start.elapsed().as_millis() as u64;
                                    let trigger_timestamp = chrono::Utc::now().to_rfc3339();
                                    
                                    if let Err(e) = emit_result {
                                        tracing::error!(
                                            "Failed to emit event {} from websocket {}: {}",
                                            triggered_event.event_name,
                                            ws_name,
                                            e
                                        );
                                    }
                                    
                                    triggered_events.push(crate::trace::TriggeredEventInfo {
                                        event_name: triggered_event.event_name.clone(),
                                        timestamp: trigger_timestamp,
                                        duration_ms: trigger_duration,
                                    });
                                }
                                // Add auto-triggered events from WebSocket config
                                for trigger in &ws_config.triggers {
                                    if exec_result.auto_trigger_payloads.contains_key(trigger) {
                                        let trigger_start = std::time::Instant::now();
                                        let payload = exec_result.auto_trigger_payloads.get(trigger).cloned();
                                        
                                        if let Some(payload) = payload {
                                            // Emit the event and measure duration
                                            let emit_result = state.event_bus.emit(trigger, payload).await;
                                            let trigger_duration = trigger_start.elapsed().as_millis() as u64;
                                            let trigger_timestamp = chrono::Utc::now().to_rfc3339();
                                            
                                            if let Err(e) = emit_result {
                                                tracing::error!(
                                                    "Failed to emit auto-triggered event {} from websocket {}: {}",
                                                    trigger,
                                                    ws_name,
                                                    e
                                                );
                                            }
                                            
                                            triggered_events.push(crate::trace::TriggeredEventInfo {
                                                event_name: trigger.clone(),
                                                timestamp: trigger_timestamp,
                                                duration_ms: trigger_duration,
                                            });
                                        }
                                    }
                                }
                            }
                            
                            state
                                .trace_store
                                .add_step_with_triggers(
                                    &message_trace_id,
                                    handler_name.clone(),
                                    duration_ms.max(exec_result.execution_time_ms),
                                    exec_result.success,
                                    exec_result.error.clone(),
                                    triggered_events.clone(),
                                )
                                .await;
                        }

                        if let Ok(result) = result {
                            if result.success {
                                if let Some(data) = result.data {
                                    if let Ok(msg) = serde_json::to_string(&data) {
                                        tracing::debug!("Sending response message: {}", msg);
                                        if let Err(e) = sender.send(Message::Text(msg.into())).await
                                        {
                                            tracing::error!(
                                                "Failed to send response message: {}",
                                                e
                                            );
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

                        } else {
                            error!("Handler execution error: {:?}", result);
                        }
                    }
                }

                // Complete message trace
                let trace_status = if ws_config.on_message.is_empty() {
                    crate::trace::TraceStatus::Success
                } else {
                    // Check if all handlers succeeded by looking at the last result
                    crate::trace::TraceStatus::Success // Simplified - could check all results
                };
                state
                    .trace_store
                    .complete_trace(&message_trace_id, trace_status, None)
                    .await;
            }
            Ok(Message::Close(_)) => {
                break;
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    let mut disconnect_metadata = HashMap::new();
    disconnect_metadata.insert("path".to_string(), ws_config.path.clone());
    disconnect_metadata.insert("connection_id".to_string(), connection_id.clone());
    let disconnect_trace_id = state
        .trace_store
        .start_trace(
            format!("{} (disconnect)", ws_name),
            TraceEntryType::WebSocket,
            disconnect_metadata,
        )
        .await;

    if !ws_config.on_disconnect.is_empty() {
        for handler_name in &ws_config.on_disconnect {
            let handler_name = match state.config.language {
                config::Language::TypeScript => handler_name.clone(),
                config::Language::Python => templates::to_snake_case(handler_name.as_str()),
            };

            let payload = connection.clone();
            let mut context = rohas_runtime::HandlerContext::new(&handler_name, payload);
            context
                .metadata
                .insert("websocket_name".to_string(), ws_name.clone());
            
            let start = Instant::now();
            let result = state.executor.execute_with_context(context).await;
            let duration_ms = start.elapsed().as_millis() as u64;

            if let Ok(ref exec_result) = result {
                state
                    .trace_store
                    .add_step(
                        &disconnect_trace_id,
                        handler_name.clone(),
                        duration_ms.max(exec_result.execution_time_ms),
                        exec_result.success,
                        exec_result.error.clone(),
                    )
                    .await;
            }
        }
    }

    state
        .trace_store
        .complete_trace(&disconnect_trace_id, crate::trace::TraceStatus::Success, None)
        .await;
}
