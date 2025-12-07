use crate::connection::Database;
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// Trait for database models
///
/// This trait is automatically derived when using the `#[derive(Model)]` macro
pub trait Model: Serialize + for<'de> Deserialize<'de> + Send + Sync + Clone {
    fn table_name() -> &'static str;

    fn primary_key() -> &'static str;

    fn primary_key_value(&self) -> Result<Box<dyn std::any::Any + Send>>;

    async fn find_by_id(db: &Database, id: i64) -> Result<Option<Self>>;

    async fn find_all(db: &Database) -> Result<Vec<Self>>;

    async fn save(&self, db: &Database) -> Result<()>;

    async fn delete(&self, db: &Database) -> Result<()>;

    async fn create(db: &Database, data: Self) -> Result<Self>;

    async fn update(db: &Database, id: i64, data: Self) -> Result<Self>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseModel<T> {
    #[serde(skip)]
    _phantom: PhantomData<T>,
}

impl<T> BaseModel<T> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<T> Default for BaseModel<T> {
    fn default() -> Self {
        Self::new()
    }
}

