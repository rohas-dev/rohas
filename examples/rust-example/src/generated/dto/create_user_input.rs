use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserInput
{
    pub name: String,
    pub email: String,
}
