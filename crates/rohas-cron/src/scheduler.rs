use crate::error::{CronError, Result};
use crate::job::{CronJob, ExecutionRecord, JobConfig, JobStatus};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};

pub type JobHandler = Arc<
    dyn Fn(&JobConfig) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>>
        + Send
        + Sync,
>;

pub struct Scheduler {
    jobs: Arc<RwLock<HashMap<String, Arc<CronJob>>>>,
    handlers: Arc<RwLock<HashMap<String, JobHandler>>>,
    running: Arc<RwLock<bool>>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
            handlers: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn add_job(&self, config: JobConfig) -> Result<String> {
        let job = Arc::new(CronJob::new(config)?);
        let job_id = job.id().to_string();

        let mut jobs = self.jobs.write().await;
        jobs.insert(job_id.clone(), job);

        info!("Added cron job: {} ({})", job_id, jobs.len());

        Ok(job_id)
    }

    pub async fn remove_job(&self, job_id: &str) -> Result<()> {
        let mut jobs = self.jobs.write().await;

        if jobs.remove(job_id).is_some() {
            info!("Removed cron job: {}", job_id);
            Ok(())
        } else {
            Err(CronError::JobNotFound(job_id.to_string()))
        }
    }

    pub async fn register_handler<F, Fut>(&self, job_name: &str, handler: F)
    where
        F: Fn(&JobConfig) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        let mut handlers = self.handlers.write().await;

        let handler_fn: JobHandler = Arc::new(move |config| {
            let fut = handler(config);
            Box::pin(fut)
        });

        handlers.insert(job_name.to_string(), handler_fn);
        info!("Registered handler for job: {}", job_name);
    }

    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(CronError::SchedulerError(
                "Scheduler already running".into(),
            ));
        }

        *running = true;
        drop(running);

        info!("Starting cron scheduler");

        let jobs = self.jobs.clone();
        let handlers = self.handlers.clone();
        let running_flag = self.running.clone();

        tokio::spawn(async move {
            while *running_flag.read().await {
                Self::tick(&jobs, &handlers).await;
                sleep(Duration::from_secs(1)).await;
            }
        });

        Ok(())
    }

    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        info!("Stopped cron scheduler");
    }

    async fn tick(
        jobs: &Arc<RwLock<HashMap<String, Arc<CronJob>>>>,
        handlers: &Arc<RwLock<HashMap<String, JobHandler>>>,
    ) {
        let jobs_map = jobs.read().await;
        let handlers_map = handlers.read().await;

        for (_job_id, job) in jobs_map.iter() {
            if job.should_run().await {
                let job_name = job.name().to_string();
                debug!("Job should run: {}", job_name);

                if let Some(handler) = handlers_map.get(&job_name) {
                    let job = Arc::clone(job);
                    let handler = Arc::clone(handler);

                    tokio::spawn(async move {
                        Self::execute_job(job, handler).await;
                    });
                } else {
                    warn!("No handler registered for job: {}", job_name);
                }
            }
        }
    }

    async fn execute_job(job: Arc<CronJob>, handler: JobHandler) {
        let config = job.config();
        let mut record = ExecutionRecord::new(config.id.clone());

        info!("Executing cron job: {}", config.name);

        match tokio::time::timeout(Duration::from_secs(config.timeout_seconds), handler(config))
            .await
        {
            Ok(Ok(())) => {
                record.complete();
                info!("Job completed successfully: {}", config.name);
            }
            Ok(Err(e)) => {
                let error_msg = format!("Job failed: {}", e);
                error!("{}", error_msg);
                record.fail(error_msg);
            }
            Err(_) => {
                let error_msg = format!("Job timed out after {} seconds", config.timeout_seconds);
                error!("{}", error_msg);
                record.fail(error_msg);
            }
        }

        job.record_execution(record).await;
    }

    pub async fn list_jobs(&self) -> Vec<JobConfig> {
        let jobs = self.jobs.read().await;
        jobs.values().map(|job| job.config().clone()).collect()
    }

    pub async fn get_job_status(&self, job_id: &str) -> Result<JobStatus> {
        let jobs = self.jobs.read().await;

        if let Some(job) = jobs.get(job_id) {
            if !job.is_enabled() {
                return Ok(JobStatus::Disabled);
            }

            if let Some(record) = job.last_execution().await {
                Ok(record.status)
            } else {
                Ok(JobStatus::Scheduled)
            }
        } else {
            Err(CronError::JobNotFound(job_id.to_string()))
        }
    }

    pub async fn get_execution_record(&self, job_id: &str) -> Result<Option<ExecutionRecord>> {
        let jobs = self.jobs.read().await;

        if let Some(job) = jobs.get(job_id) {
            Ok(job.last_execution().await)
        } else {
            Err(CronError::JobNotFound(job_id.to_string()))
        }
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scheduler_lifecycle() {
        let scheduler = Scheduler::new();

        let config = JobConfig::new("test_job", "0 0 0 * * *");
        let job_id = scheduler.add_job(config).await.unwrap();

        let jobs = scheduler.list_jobs().await;
        assert_eq!(jobs.len(), 1);

        scheduler.remove_job(&job_id).await.unwrap();

        let jobs = scheduler.list_jobs().await;
        assert_eq!(jobs.len(), 0);
    }

    #[tokio::test]
    async fn test_handler_registration() {
        let scheduler = Scheduler::new();

        scheduler
            .register_handler("test_job", |_config| async { Ok(()) })
            .await;

        let handlers = scheduler.handlers.read().await;
        assert!(handlers.contains_key("test_job"));
    }
}
