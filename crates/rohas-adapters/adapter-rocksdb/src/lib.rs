use rohas_telemetry::error::{Result, TelemetryError};
use rohas_telemetry::storage::{IterateCallback, StorageAdapter};
use async_trait::async_trait;
use rocksdb::{DB, IteratorMode, Options};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// RocksDB storage adapter for telemetry data
pub struct RocksDBAdapter {
    db: Arc<RwLock<DB>>,
}

impl RocksDBAdapter {
    pub async fn new(path: PathBuf) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| TelemetryError::Io(e))?;
        }

        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        
        opts.set_write_buffer_size(64 * 1024 * 1024); // 64MB
        opts.set_max_write_buffer_number(3);
        opts.set_target_file_size_base(64 * 1024 * 1024); // 64MB
        
        opts.set_compression_type(rocksdb::DBCompressionType::Lz4);
        
        opts.optimize_for_point_lookup(1024);

        let db = DB::open(&opts, &path)
            .map_err(|e| TelemetryError::StorageBackend(e.to_string()))?;

        Ok(Self {
            db: Arc::new(RwLock::new(db)),
        })
    }
}

#[async_trait]
impl StorageAdapter for RocksDBAdapter {
    async fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        let db = self.db.write().await;
        db.put(key, value)
            .map_err(|e| TelemetryError::StorageBackend(e.to_string()))?;
        Ok(())
    }

    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let db = self.db.read().await;
        match db.get(key) {
            Ok(Some(value)) => Ok(Some(value)),
            Ok(None) => Ok(None),
            Err(e) => Err(TelemetryError::StorageBackend(e.to_string())),
        }
    }

    async fn delete(&self, key: &[u8]) -> Result<()> {
        let db = self.db.write().await;
        db.delete(key)
            .map_err(|e| TelemetryError::StorageBackend(e.to_string()))?;
        Ok(())
    }

    async fn get_by_prefix(&self, prefix: &[u8]) -> Result<Vec<Vec<u8>>> {
        let db = self.db.read().await;
        let iter = db.iterator(IteratorMode::From(prefix, rocksdb::Direction::Forward));
        
        let mut keys = Vec::new();
        for item in iter {
            let (key, _) = item.map_err(|e| TelemetryError::StorageBackend(e.to_string()))?;
            if key.starts_with(prefix) {
                keys.push(key.to_vec());
            } else {
                break;
            }
        }
        
        Ok(keys)
    }

    async fn iterate(&self, prefix: &[u8], mut callback: Box<dyn IterateCallback>) -> Result<()> {
        let db = self.db.read().await;
        let iter = db.iterator(IteratorMode::From(prefix, rocksdb::Direction::Forward));
        
        for item in iter {
            let (key, value) = item.map_err(|e| TelemetryError::StorageBackend(e.to_string()))?;
            if key.starts_with(prefix) {
                let should_continue = callback.call(&key, &value)?;
                if !should_continue {
                    break;
                }
            } else {
                break;
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_rocksdb_adapter() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_db");
        
        let adapter = RocksDBAdapter::new(db_path).await.unwrap();
        
        adapter.put(b"test:key1", b"value1").await.unwrap();
        let value = adapter.get(b"test:key1").await.unwrap();
        assert_eq!(value, Some(b"value1".to_vec()));
        
        adapter.delete(b"test:key1").await.unwrap();
        let value = adapter.get(b"test:key1").await.unwrap();
        assert_eq!(value, None);
        
        adapter.put(b"test:key1", b"value1").await.unwrap();
        adapter.put(b"test:key2", b"value2").await.unwrap();
        adapter.put(b"other:key1", b"value3").await.unwrap();
        
        let keys = adapter.get_by_prefix(b"test:").await.unwrap();
        assert_eq!(keys.len(), 2);
    }
}

