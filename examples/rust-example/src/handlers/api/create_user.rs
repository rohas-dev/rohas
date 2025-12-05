use crate::generated::api::create_user::{ CreateUserRequest, CreateUserResponse };
use crate::generated::state::State;
use rohas_runtime::Result;

/// High-performance Rust handler for CreateUser API.
pub async fn handle_create_user(
    req: CreateUserRequest,
    state: &mut State,
) -> Result<CreateUserResponse> {
    state.logger().info("CreateUser handler called");
    state.logger().info(&format!("Request: {:?}", req));
    state.logger().info(&format!("State: {:?}", state));

    // Create a new user from the request
    Ok(CreateUserResponse {
        id: 1, // TODO: Generate proper ID
        name: req.name.clone(),
        email: req.email.clone(),
        createdAt: chrono::Utc::now(),
    })
}
