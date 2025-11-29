use crate::storage::StorageAdapter;

pub struct TelemetryAdapter {
    storage: Box<dyn StorageAdapter>,
}

impl TelemetryAdapter {
    
    pub fn new(storage: Box<dyn StorageAdapter>) -> Self {
        Self { storage }
    }

    pub fn storage(&self) -> &dyn StorageAdapter {
        self.storage.as_ref()
    }
}

