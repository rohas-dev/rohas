use crate::generated::events::user_created::UserCreated;
use rohas_runtime::{HandlerContext, HandlerResult, Result};

/// High-performance Rust event handler.
pub async fn send_welcome_email(
    event: UserCreated,
) -> Result<HandlerResult> {
    // TODO: Implement event handler
    tracing::info!("Handling event: {:?}", event);
    Ok(HandlerResult::success(serde_json::json!({}), 0))
}
