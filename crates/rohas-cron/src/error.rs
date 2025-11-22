use thiserror::Error;

pub type Result<T> = std::result::Result<T, CronError>;

#[derive(Error, Debug)]
pub enum CronError {
    #[error("Invalid cron expression: {0}")]
    InvalidExpression(String),

    #[error("Job not found: {0}")]
    JobNotFound(String),

    #[error("Job execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Scheduler error: {0}")]
    SchedulerError(String),

    #[error("Parse error: {0}")]
    ParseError(#[from] cron::error::Error),
}
