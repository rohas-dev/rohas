pub mod adapter;
pub mod error;
pub mod logs;
pub mod metrics;
pub mod storage;
pub mod traces;

pub use adapter::TelemetryAdapter;
pub use error::{Result, TelemetryError};
pub use logs::{LogEntry, LogStore};
pub use metrics::{Metric, MetricStore, MetricType};
pub use storage::StorageAdapter;
pub use traces::{TraceEntry, TraceStore};

