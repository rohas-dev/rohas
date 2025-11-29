use crate::error::Result;
use crate::storage::StorageAdapter;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEntry {
    pub id: String,
    pub entry_point: String,
    pub entry_type: String, // "api", "event", "cron", "websocket"
    pub status: String,    // "success", "failed", "running"
    pub duration_ms: u64,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub steps: Vec<TraceStep>,
    pub error: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceStep {
    pub name: String,
    pub handler_name: String,
    pub duration_ms: u64,
    pub success: bool,
    pub error: Option<String>,
    pub timestamp: String,
    pub triggered_events: Vec<TriggeredEventInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggeredEventInfo {
    pub event_name: String,
    pub timestamp: String,
    pub duration_ms: u64,
}

impl TraceEntry {
    fn key(&self) -> Vec<u8> {
        format!("trace:{}:{}", self.started_at, self.id).into_bytes()
    }

    fn prefix() -> &'static [u8] {
        b"trace:"
    }

    fn id_key(id: &str) -> Vec<u8> {
        format!("trace:id:{}", id).into_bytes()
    }
}

pub struct TraceStore {
    storage: Arc<dyn StorageAdapter>,
}

impl TraceStore {
    pub fn new(storage: Arc<dyn StorageAdapter>) -> Self {
        Self { storage }
    }

    pub async fn store(&self, entry: TraceEntry) -> Result<()> {
        let key = entry.key();
        let value = serde_json::to_vec(&entry)?;
        self.storage.put(&key, &value).await?;

        let id_key = TraceEntry::id_key(&entry.id);
        self.storage.put(&id_key, &value).await?;

        Ok(())
    }

    pub async fn get_by_id(&self, id: &str) -> Result<Option<TraceEntry>> {
        let key = TraceEntry::id_key(id);
        match self.storage.get(&key).await? {
            Some(value) => {
                let entry: TraceEntry = serde_json::from_slice(&value)?;
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
    ) -> Result<Vec<TraceEntry>> {
        use std::sync::{Arc, Mutex};
        
        let entries = Arc::new(Mutex::new(Vec::new()));
        let start_key = format!("trace:{}:", start_time.to_rfc3339()).into_bytes();
        let end_key = format!("trace:{}:", end_time.to_rfc3339()).into_bytes();
        let limit = limit.unwrap_or(usize::MAX);

        let entries_clone = entries.clone();
        let start_key_clone = start_key.clone();
        let end_key_clone = end_key.clone();

        struct TraceCallback {
            entries: Arc<std::sync::Mutex<Vec<TraceEntry>>>,
            start_key: Vec<u8>,
            end_key: Vec<u8>,
            limit: usize,
        }
        
        impl crate::storage::IterateCallback for TraceCallback {
            fn call(&mut self, key: &[u8], value: &[u8]) -> Result<bool> {
                if key >= self.start_key.as_slice() && key <= self.end_key.as_slice() {
                    if !key.starts_with(b"trace:id:") {
                        if let Ok(entry) = serde_json::from_slice::<TraceEntry>(value) {
                            let mut entries = self.entries.lock().unwrap();
                            entries.push(entry);
                            if entries.len() >= self.limit {
                                return Ok(false); 
                            }
                        }
                    }
                }
                Ok(true)  
            }
        }

        self.storage
            .iterate(
                TraceEntry::prefix(),
                Box::new(TraceCallback {
                    entries: entries_clone,
                    start_key: start_key_clone,
                    end_key: end_key_clone,
                    limit,
                }),
            )
            .await?;

        let mut result = entries.lock().unwrap().clone();
        result.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        Ok(result)
    }

    pub async fn get_recent(&self, limit: usize) -> Result<Vec<TraceEntry>> {
        let end_time = Utc::now();
        let start_time = end_time - chrono::Duration::hours(24);
        self.query_range(start_time, end_time, Some(limit)).await
    }

    pub async fn get_all(&self, limit: Option<usize>) -> Result<Vec<TraceEntry>> {
        use std::sync::{Arc, Mutex};
        
        let entries = Arc::new(Mutex::new(Vec::new()));
        let limit = limit.unwrap_or(usize::MAX);

        struct TraceCallback {
            entries: Arc<std::sync::Mutex<Vec<TraceEntry>>>,
            limit: usize,
        }
        
        impl crate::storage::IterateCallback for TraceCallback {
            fn call(&mut self, key: &[u8], value: &[u8]) -> Result<bool> {
                if !key.starts_with(b"trace:id:") {
                    if let Ok(entry) = serde_json::from_slice::<TraceEntry>(value) {
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
                TraceEntry::prefix(),
                Box::new(TraceCallback {
                    entries: entries.clone(),
                    limit,
                }),
            )
            .await?;

        let mut result = entries.lock().unwrap().clone();
        result.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        Ok(result)
    }

    pub async fn delete_older_than(&self, before: DateTime<Utc>) -> Result<usize> {
        use std::sync::{Arc, Mutex};
        
        let keys_to_delete = Arc::new(Mutex::new(Vec::new()));
        let cutoff_key = format!("trace:{}:", before.to_rfc3339()).into_bytes();
        let cutoff_key_clone = cutoff_key.clone();

        struct DeleteCallback {
            keys_to_delete: Arc<Mutex<Vec<Vec<u8>>>>,
            cutoff_key: Vec<u8>,
        }
        
        impl crate::storage::IterateCallback for DeleteCallback {
            fn call(&mut self, key: &[u8], _value: &[u8]) -> Result<bool> {
                if !key.starts_with(b"trace:id:") {
                    if key < self.cutoff_key.as_slice() {
                        let mut keys = self.keys_to_delete.lock().unwrap();
                        keys.push(key.to_vec());
                    }
                }
                Ok(true)  
            }
        }

        self.storage
            .iterate(
                TraceEntry::prefix(),
                Box::new(DeleteCallback {
                    keys_to_delete: keys_to_delete.clone(),
                    cutoff_key: cutoff_key_clone,
                }),
            )
            .await?;
        
        let keys = keys_to_delete.lock().unwrap().clone();
        let mut deleted_count = 0;
        
        for key in &keys {
            if let Err(e) = self.storage.delete(key).await {
                tracing::warn!("Failed to delete trace key: {:?}, error: {}", key, e);
            } else {
                deleted_count += 1;
            }
        }
        
        for key in &keys {
            if let Some(id_start) = key.iter().rposition(|&b| b == b':') {
                if let Ok(id_str) = std::str::from_utf8(&key[id_start + 1..]) {
                    let id_key = TraceEntry::id_key(id_str);
                    let _ = self.storage.delete(&id_key).await;
                }
            }
        }

        Ok(deleted_count)
    }
}

