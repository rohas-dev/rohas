use rohas_runtime::{HandlerContext, HandlerResult, Result};
use crate::generated::state::State;

/// High-performance Rust middleware.
pub async fn request_logging_middleware(
    ctx: HandlerContext,
    state: &mut State,
) -> Result<HandlerResult> {
    // TODO: Implement middleware logic
    // Return Ok to continue, Err to abort
    tracing::info!("Middleware request_logging executed");
    Ok(HandlerResult::success(serde_json::json!({}), 0))
}
