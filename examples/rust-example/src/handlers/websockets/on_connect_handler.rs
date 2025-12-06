use crate::generated::websockets::hello_world_w_s::HelloWorldWSConnection;
use crate::generated::websockets::hello_world_w_s::HelloWorldWSMessage;
use rohas_runtime::{HandlerContext, HandlerResult, Result};
use crate::generated::state::State;

/// Rust WebSocket connect handler.
pub async fn on_connect_handler(
    connection: HelloWorldWSConnection,
    state: &mut State,
) -> Result<HandlerResult> {
    tracing::info!("WebSocket connect handler: {:?}", connection);
    Ok(HandlerResult::success(serde_json::json!({}), 0))
}
