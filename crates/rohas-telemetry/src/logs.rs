use crate::error::Result;
use crate::storage::StorageAdapter;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: String,
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
    pub fields: serde_json::Value,
    pub span_name: Option<String>,
    pub span_fields: serde_json::Value,
    pub file: Option<String>,
    pub line: Option<u32>,
}

impl LogEntry {
    fn key(&self) -> Vec<u8> {
        format!("log:{}:{}", self.timestamp, self.id).into_bytes()
    }

    fn prefix() -> &'static [u8] {
        b"log:"
    }
}

pub struct LogStore {
    storage: Arc<dyn StorageAdapter>,
}

impl LogStore {
    pub fn new(storage: Arc<dyn StorageAdapter>) -> Self {
        Self { storage }
    }

    pub async fn store(&self, entry: LogEntry) -> Result<()> {
        let key = entry.key();
        let value = serde_json::to_vec(&entry)?;
        self.storage.put(&key, &value).await
    }

    pub async fn get(&self, timestamp: &str, id: &str) -> Result<Option<LogEntry>> {
        let key = format!("log:{}:{}", timestamp, id).into_bytes();
        match self.storage.get(&key).await? {
            Some(value) => {
                let entry: LogEntry = serde_json::from_slice(&value)?;
                Ok(Some(entry))
            }
            None => Ok(None),
        }
    }

    pub async fn query_range(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: Option<usize>,
    ) -> Result<Vec<LogEntry>> {
        use std::sync::{Arc, Mutex};
        
        let entries = Arc::new(Mutex::new(Vec::new()));
        let start_key = format!("log:{}:", start_time.to_rfc3339()).into_bytes();
        let end_key = format!("log:{}:", end_time.to_rfc3339()).into_bytes();
        let limit = limit.unwrap_or(usize::MAX);

        let entries_clone = entries.clone();
        let start_key_clone = start_key.clone();
        let end_key_clone = end_key.clone();
        
        struct LogCallback {
            entries: Arc<std::sync::Mutex<Vec<LogEntry>>>,
            start_key: Vec<u8>,
            end_key: Vec<u8>,
            limit: usize,
        }
        
        impl crate::storage::IterateCallback for LogCallback {
            fn call(&mut self, key: &[u8], value: &[u8]) -> Result<bool> {
                if key >= self.start_key.as_slice() && key <= self.end_key.as_slice() {
                    if let Ok(entry) = serde_json::from_slice::<LogEntry>(value) {
                        let mut entries = self.entries.lock().unwrap();
                        entries.push(entry);
                        if entries.len() >= self.limit {
                            return Ok(false); 
                        }
                    }
                }
                Ok(true)  
            }
        }
        
        self.storage
            .iterate(
                LogEntry::prefix(),
                Box::new(LogCallback {
                    entries: entries_clone,
                    start_key: start_key_clone,
                    end_key: end_key_clone,
                    limit,
                }),
            )
            .await?;

        let mut result = entries.lock().unwrap().clone();
        result.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(result)
    }

    pub async fn delete_older_than(&self, before: DateTime<Utc>) -> Result<usize> {
        use std::sync::{Arc, Mutex};
        
        let count = Arc::new(Mutex::new(0));
        let cutoff_key = format!("log:{}:", before.to_rfc3339()).into_bytes();
        let cutoff_key_clone = cutoff_key.clone();

        struct DeleteCallback {
            count: Arc<Mutex<usize>>,
            cutoff_key: Vec<u8>,
        }
        
        impl crate::storage::IterateCallback for DeleteCallback {
            fn call(&mut self, key: &[u8], _value: &[u8]) -> Result<bool> {
                if key < self.cutoff_key.as_slice() {
                    let mut count = self.count.lock().unwrap();
                    *count += 1;
                }
                Ok(true)  
            }
        }

        self.storage
            .iterate(
                LogEntry::prefix(),
                Box::new(DeleteCallback {
                    count: count.clone(),
                    cutoff_key: cutoff_key_clone,
                }),
            )
            .await?;
        
        let result = *count.lock().unwrap();

        // TODO: Implement batch deletion
        Ok(result)
    }
}

