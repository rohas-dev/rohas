use crate::generated::websockets::hello_world_w_s::HelloWorldWSConnection;
use crate::generated::websockets::hello_world_w_s::HelloWorldWSMessage;
use rohas_runtime::{HandlerContext, HandlerResult, Result};
use crate::generated::state::State;

/// Rust WebSocket disconnect handler.
pub async fn on_disconnect_handler(
    connection: HelloWorldWSConnection,
) -> Result<HandlerResult> {
    tracing::info!("WebSocket disconnect handler: {:?}", connection);
    Ok(HandlerResult::success(serde_json::json!({}), 0))
}
