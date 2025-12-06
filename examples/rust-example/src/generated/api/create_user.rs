use serde::{Deserialize, Serialize};
use super::super::dto::create_user_input::CreateUserInput;
use super::super::models::user::User;

pub type CreateUserRequest = CreateUserInput;

pub type CreateUserResponse = User;
