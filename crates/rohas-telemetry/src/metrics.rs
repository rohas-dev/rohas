use crate::error::Result;
use crate::storage::StorageAdapter;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub id: String,
    pub name: String,
    pub metric_type: MetricType,
    pub value: f64,
    pub timestamp: String,
    pub labels: HashMap<String, String>,
    pub unit: Option<String>,
}

impl Metric {
    fn key(&self) -> Vec<u8> {
        format!("metric:{}:{}:{}", self.name, self.timestamp, self.id).into_bytes()
    }

    fn name_prefix(name: &str) -> Vec<u8> {
        format!("metric:{}:", name).into_bytes()
    }
}

pub struct MetricStore {
    storage: Arc<dyn StorageAdapter>,
}

impl MetricStore {
    pub fn new(storage: Arc<dyn StorageAdapter>) -> Self {
        Self { storage }
    }

    pub async fn store(&self, metric: Metric) -> Result<()> {
        let key = metric.key();
        let value = serde_json::to_vec(&metric)?;
        self.storage.put(&key, &value).await
    }

    pub async fn query(
        &self,
        name: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: Option<usize>,
    ) -> Result<Vec<Metric>> {
        use std::sync::{Arc, Mutex};
        
        let metrics = Arc::new(Mutex::new(Vec::new()));
        let prefix = Metric::name_prefix(name);
        let start_key = format!("metric:{}:{}:", name, start_time.to_rfc3339()).into_bytes();
        let end_key = format!("metric:{}:{}:", name, end_time.to_rfc3339()).into_bytes();
        let limit = limit.unwrap_or(usize::MAX);

        let metrics_clone = metrics.clone();
        let start_key_clone = start_key.clone();
        let end_key_clone = end_key.clone();

        struct MetricCallback {
            metrics: Arc<std::sync::Mutex<Vec<Metric>>>,
            start_key: Vec<u8>,
            end_key: Vec<u8>,
            limit: usize,
        }
        
        impl crate::storage::IterateCallback for MetricCallback {
            fn call(&mut self, key: &[u8], value: &[u8]) -> Result<bool> {
                if key >= self.start_key.as_slice() && key <= self.end_key.as_slice() {
                    if let Ok(metric) = serde_json::from_slice::<Metric>(value) {
                        let mut metrics = self.metrics.lock().unwrap();
                        metrics.push(metric);
                        if metrics.len() >= self.limit {
                            return Ok(false); 
                        }
                    }
                }
                Ok(true)  
            }
        }

        self.storage
            .iterate(
                &prefix,
                Box::new(MetricCallback {
                    metrics: metrics_clone,
                    start_key: start_key_clone,
                    end_key: end_key_clone,
                    limit,
                }),
            )
            .await?;

        let mut result = metrics.lock().unwrap().clone();
        result.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(result)
    }

    pub async fn get_latest(&self, name: &str) -> Result<Option<Metric>> {
        let end_time = Utc::now();
        let start_time = end_time - chrono::Duration::hours(24);
        let metrics = self.query(name, start_time, end_time, Some(1)).await?;
        Ok(metrics.into_iter().next())
    }

    pub async fn aggregate(
        &self,
        name: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<MetricAggregation> {
        let metrics = self.query(name, start_time, end_time, None).await?;
        
        if metrics.is_empty() {
            return Ok(MetricAggregation {
                count: 0,
                sum: 0.0,
                avg: 0.0,
                min: 0.0,
                max: 0.0,
            });
        }

        let values: Vec<f64> = metrics.iter().map(|m| m.value).collect();
        let sum: f64 = values.iter().sum();
        let count = values.len();
        let avg = sum / count as f64;
        let min = values
            .iter()
            .copied()
            .reduce(f64::min)
            .unwrap_or(0.0);
        let max = values
            .iter()
            .copied()
            .reduce(f64::max)
            .unwrap_or(0.0);

        Ok(MetricAggregation {
            count,
            sum,
            avg,
            min,
            max,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricAggregation {
    pub count: usize,
    pub sum: f64,
    pub avg: f64,
    pub min: f64,
    pub max: f64,
}

