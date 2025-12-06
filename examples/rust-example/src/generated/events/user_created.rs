use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use crate::generated::models::user::User;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCreated
{
    pub payload: User,
    pub timestamp: DateTime<Utc>,
}
