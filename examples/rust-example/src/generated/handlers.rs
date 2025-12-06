// Auto-generated handler registration
// DO NOT EDIT MANUALLY

use rohas_runtime::{RustRuntime, HandlerContext, HandlerResult, Result};
use std::sync::Arc;
use std::sync::OnceLock;

// Global registry for automatic handler registration
static RUNTIME_REGISTRY: OnceLock<Arc<RustRuntime>> = OnceLock::new();

/// Set the runtime for automatic handler registration.
/// This is called automatically by the engine.
/// This function is public so it can be called from the engine.
/// Note: Each dylib has its own OnceLock, so this can be called fresh on each reload.
pub fn set_runtime(runtime: Arc<RustRuntime>) {
    // Set the runtime (this will only succeed once per dylib load, which is what we want)
    let _ = RUNTIME_REGISTRY.set(runtime);
    // Always trigger registration (important for hot reload)
    register_all_handlers_internal().expect("Failed to register handlers");
}

// Import handler functions
use crate::handlers::api::create_user::handle_create_user;
use crate::handlers::api::hello_world::handle_hello_world;
use crate::handlers::events::send_welcome_email::send_welcome_email;

/// Register all handlers with the Rust runtime.
/// This function should be called during engine initialization.
pub async fn register_all_handlers(runtime: Arc<RustRuntime>) -> Result<()> {
    set_runtime(runtime);
    Ok(())
}

/// Internal registration function (synchronous, for static initialization).
fn register_all_handlers_internal() -> Result<()> {
    use tracing::info;
    info!("Registering Rust handlers from dylib...");
    let runtime = RUNTIME_REGISTRY.get().ok_or_else(|| rohas_runtime::RuntimeError::ExecutionFailed("Runtime not set".into()))?;
    let rt = tokio::runtime::Runtime::new().map_err(|e| rohas_runtime::RuntimeError::ExecutionFailed(e.to_string()))?;
    rt.block_on(async {
        // Register API handler: CreateUser
        runtime.register_handler(
            "create_user".to_string(),
            |ctx: HandlerContext| async move {
                // Parse request from context
                let req: crate::generated::api::create_user::CreateUserRequest = serde_json::from_value(ctx.payload.clone())?;
                let mut state = crate::generated::state::State::new(&ctx.handler_name);
                let response = handle_create_user(req, &mut state).await?;
                Ok(HandlerResult::success(serde_json::to_value(response)?, 0))
            }
        ).await;
        info!("Registered handler: create_user");
        // Register API handler: HelloWorld
        runtime.register_handler(
            "hello_world".to_string(),
            |ctx: HandlerContext| async move {
                // Parse request from context
                let req: crate::generated::api::hello_world::HelloWorldRequest = serde_json::from_value(ctx.payload.clone())?;
                let mut state = crate::generated::state::State::new(&ctx.handler_name);
                let response = handle_hello_world(req, &mut state).await?;
                Ok(HandlerResult::success(serde_json::to_value(response)?, 0))
            }
        ).await;
        info!("Registered handler: hello_world");
        Ok::<(), rohas_runtime::RuntimeError>(())
    })?;
    Ok(())
}


