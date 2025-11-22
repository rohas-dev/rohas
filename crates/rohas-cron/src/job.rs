use crate::error::{CronError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Job configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobConfig {
    /// Unique job ID
    pub id: String,

    /// Job name
    pub name: String,

    /// Cron expression (e.g., "0 0 * * *")
    pub schedule: String,

    /// Whether the job is enabled
    pub enabled: bool,

    /// Maximum execution time in seconds
    pub timeout_seconds: u64,

    /// Events to trigger after execution
    pub triggers: Vec<String>,
}

impl JobConfig {
    pub fn new(name: impl Into<String>, schedule: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            schedule: schedule.into(),
            enabled: true,
            timeout_seconds: 300, // 5 minutes default
            triggers: Vec::new(),
        }
    }

    pub fn with_triggers(mut self, triggers: Vec<String>) -> Self {
        self.triggers = triggers;
        self
    }

    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }

    /// Validate the cron expression
    pub fn validate(&self) -> Result<()> {
        use std::str::FromStr;
        cron::Schedule::from_str(&self.schedule)
            .map_err(|e| CronError::InvalidExpression(e.to_string()))?;
        Ok(())
    }
}

/// Job status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
    Scheduled,
    Running,
    Completed,
    Failed,
    Disabled,
}

/// Execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub job_id: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: JobStatus,
    pub error: Option<String>,
    pub duration_ms: Option<u64>,
}

impl ExecutionRecord {
    pub fn new(job_id: String) -> Self {
        Self {
            job_id,
            started_at: Utc::now(),
            completed_at: None,
            status: JobStatus::Running,
            error: None,
            duration_ms: None,
        }
    }

    pub fn complete(&mut self) {
        self.completed_at = Some(Utc::now());
        self.status = JobStatus::Completed;
        self.duration_ms =
            Some((self.completed_at.unwrap() - self.started_at).num_milliseconds() as u64);
    }

    pub fn fail(&mut self, error: String) {
        self.completed_at = Some(Utc::now());
        self.status = JobStatus::Failed;
        self.error = Some(error);
        self.duration_ms =
            Some((self.completed_at.unwrap() - self.started_at).num_milliseconds() as u64);
    }
}

/// Cron job definition
pub struct CronJob {
    config: JobConfig,
    schedule: cron::Schedule,
    last_execution: Arc<RwLock<Option<ExecutionRecord>>>,
    next_run: Arc<RwLock<Option<DateTime<Utc>>>>,
}

impl CronJob {
    pub fn new(config: JobConfig) -> Result<Self> {
        use std::str::FromStr;
        config.validate()?;

        let schedule = cron::Schedule::from_str(&config.schedule)
            .map_err(|e| CronError::InvalidExpression(e.to_string()))?;

        let next_run = schedule.upcoming(Utc).next();

        Ok(Self {
            config,
            schedule,
            last_execution: Arc::new(RwLock::new(None)),
            next_run: Arc::new(RwLock::new(next_run)),
        })
    }

    pub fn config(&self) -> &JobConfig {
        &self.config
    }

    pub fn id(&self) -> &str {
        &self.config.id
    }

    pub fn name(&self) -> &str {
        &self.config.name
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub async fn next_run(&self) -> Option<DateTime<Utc>> {
        *self.next_run.read().await
    }

    pub async fn update_next_run(&self) {
        let next = self.schedule.upcoming(Utc).next();
        *self.next_run.write().await = next;
    }

    pub async fn last_execution(&self) -> Option<ExecutionRecord> {
        self.last_execution.read().await.clone()
    }

    pub async fn record_execution(&self, record: ExecutionRecord) {
        *self.last_execution.write().await = Some(record);
        self.update_next_run().await;
    }

    pub async fn should_run(&self) -> bool {
        if !self.is_enabled() {
            return false;
        }

        if let Some(next_run) = self.next_run().await {
            Utc::now() >= next_run
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_config_validation() {
        let config = JobConfig::new("test_job", "0 0 0 * * *");
        assert!(config.validate().is_ok());

        let invalid_config = JobConfig::new("test_job", "invalid");
        assert!(invalid_config.validate().is_err());
    }

    #[tokio::test]
    async fn test_cron_job_creation() {
        let config = JobConfig::new("test_job", "0 0 0 * * *");
        let job = CronJob::new(config).unwrap();

        assert_eq!(job.name(), "test_job");
        assert!(job.is_enabled());
        assert!(job.next_run().await.is_some());
    }

    #[test]
    fn test_execution_record() {
        let mut record = ExecutionRecord::new("job-123".to_string());
        assert_eq!(record.status, JobStatus::Running);

        record.complete();
        assert_eq!(record.status, JobStatus::Completed);
        assert!(record.completed_at.is_some());
        assert!(record.duration_ms.is_some());
    }
}
