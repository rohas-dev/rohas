use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloWorldRequest
{
    // No body fields
}

pub type HelloWorldResponse = String;
