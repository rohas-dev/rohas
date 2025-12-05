use crate::generated::api::hello_world::{ HelloWorldRequest, HelloWorldResponse };
use crate::generated::state::State;
use rohas_runtime::{HandlerContext, HandlerResult, Result};

/// High-performance Rust handler for HelloWorld API.
pub async fn handle_hello_world(
    req: HelloWorldRequest,
    state: &mut State,
) -> Result<HelloWorldResponse> {
    state.logger().info("Hello Worlds");
    Ok("hello  qqwqwqwsadsd".to_string())
}

