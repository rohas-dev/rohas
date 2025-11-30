use async_trait::async_trait;
use crate::error::Result;

pub trait IterateCallback: Send + Sync {
    fn call(&mut self, key: &[u8], value: &[u8]) -> Result<bool>;
}

impl<F> IterateCallback for F
where
    F: FnMut(&[u8], &[u8]) -> Result<bool> + Send + Sync,
{
    fn call(&mut self, key: &[u8], value: &[u8]) -> Result<bool> {
        self(key, value)
    }
}

#[async_trait]
pub trait StorageAdapter: Send + Sync {
    async fn put(&self, key: &[u8], value: &[u8]) -> Result<()>;

    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;

    async fn delete(&self, key: &[u8]) -> Result<()>;

    async fn get_by_prefix(&self, prefix: &[u8]) -> Result<Vec<Vec<u8>>>;

    async fn iterate(&self, prefix: &[u8], mut callback: Box<dyn IterateCallback>) -> Result<()>;

    async fn exists(&self, key: &[u8]) -> Result<bool> {
        Ok(self.get(key).await?.is_some())
    }
}

