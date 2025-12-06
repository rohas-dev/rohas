use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User
{
    pub id: i64,
    pub name: String,
    pub email: String,
    pub createdAt: chrono::DateTime<chrono::Utc>,
}
