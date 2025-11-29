use crate::api;
use crate::config::EngineConfig;
use crate::error::{EngineError, Result};
use crate::event::EventBus;
use crate::router;
use adapter_memory::MemoryAdapter;
use rohas_cron::{JobConfig, Scheduler};
use rohas_parser::{Parser, Schema};
use rohas_runtime::{Executor, RuntimeConfig};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

pub struct Engine {
    config: EngineConfig,
    schema: Arc<Schema>,
    executor: Arc<Executor>,
    event_bus: Arc<EventBus>,
    scheduler: Arc<Scheduler>,
    adapter: Arc<MemoryAdapter>,
    trace_store: Arc<crate::telemetry::TraceStore>,
    tracing_log_store: Arc<crate::tracing_log::TracingLogStore>,
    telemetry: Arc<crate::telemetry::TelemetryManager>,
    initialized: Arc<RwLock<bool>>,
}

impl Engine {
    pub async fn from_schema_file(schema_path: PathBuf, config: EngineConfig) -> Result<Self> {
        info!("Loading schema from: {}", schema_path.display());

        let schema = Parser::parse_file(&schema_path)?;
        Self::from_schema(schema, config).await
    }

    pub async fn from_schema(schema: Schema, config: EngineConfig) -> Result<Self> {
        info!("Initializing Rohas engine");

        schema.validate()?;

        let schema = Arc::new(schema);

        let runtime_config = RuntimeConfig {
            language: config.language.clone().into(),
            project_root: config.project_root.clone(),
            timeout_seconds: 30,
        };

        let executor = Arc::new(Executor::new(runtime_config));

        let telemetry_path = if config.telemetry.path.starts_with('/') {
            PathBuf::from(&config.telemetry.path)
        } else {
            config.project_root.join(&config.telemetry.path)
        };
        
        let telemetry = match config.telemetry.adapter_type {
            crate::config::TelemetryAdapterType::RocksDB => {
                Arc::new(
                    crate::telemetry::TelemetryManager::new(telemetry_path, config.telemetry.retention_days)
                        .await
                        .map_err(|e| EngineError::Initialization(e.to_string()))?
                )
            }
            crate::config::TelemetryAdapterType::Prometheus => {
                return Err(EngineError::Initialization("Prometheus adapter not yet implemented".to_string()));
            }
            crate::config::TelemetryAdapterType::InfluxDB => {
                return Err(EngineError::Initialization("InfluxDB adapter not yet implemented".to_string()));
            }
            crate::config::TelemetryAdapterType::TimescaleDB => {
                return Err(EngineError::Initialization("TimescaleDB adapter not yet implemented".to_string()));
            }
        };
        
        let trace_store = Arc::new(crate::telemetry::TraceStore::new(telemetry.clone()));
        let tracing_log_store = Arc::new(crate::tracing_log::TracingLogStore::new(1000)); // Keep last 1000 logs

        let adapter = Arc::new(MemoryAdapter::new(config.adapter.buffer_size));

        let event_bus = Arc::new(EventBus::new(
            adapter.clone(),
            executor.clone(),
            schema.clone(),
            trace_store.clone(),
        ));

        let scheduler = Arc::new(Scheduler::new());

        Ok(Self {
            config,
            schema,
            executor,
            event_bus,
            scheduler,
            adapter,
            trace_store,
            tracing_log_store,
            telemetry,
            initialized: Arc::new(RwLock::new(false)),
        })
    }

    pub async fn initialize(&self) -> Result<()> {
        let mut initialized = self.initialized.write().await;
        if *initialized {
            warn!("Engine already initialized");
            return Ok(());
        }

        info!("Initializing engine components");

        self.event_bus.initialize().await?;

        if self.config.telemetry.retention_days > 0 {
            let telemetry = self.telemetry.clone();
            let retention_days = self.config.telemetry.retention_days;
            tokio::spawn(async move {
                use tokio::time::{sleep, Duration};
                let cleanup_interval = Duration::from_secs(3600);
                loop {
                    sleep(cleanup_interval).await;
                    match telemetry.cleanup_old_traces().await {
                        Ok(count) => {
                            if count > 0 {
                                info!("Cleaned up {} old traces (retention: {} days)", count, retention_days);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to cleanup old traces: {}", e);
                        }
                    }
                }
            });
            info!("Started telemetry cleanup task (retention: {} days)", retention_days);
        } else {
            info!("Telemetry retention disabled (retention_days = 0), traces will be kept forever");
        }

        for cron in &self.schema.crons {
            let job_config = JobConfig::new(cron.name.clone(), cron.schedule.clone())
                .with_triggers(cron.triggers.clone());

            let job_id = self.scheduler.add_job(job_config).await?;
            info!("Registered cron job: {} ({})", cron.name, job_id);

            let cron_name = cron.name.clone();
            let cron_schedule = cron.schedule.clone();
            let executor = self.executor.clone();
            let event_bus = self.event_bus.clone();
            let triggers = cron.triggers.clone();
            let trace_store = self.trace_store.clone();

            self.scheduler
                .register_handler(&cron_name.clone(), move |_config| {
                    let executor = executor.clone();
                    let event_bus = event_bus.clone();
                    let triggers = triggers.clone();
                    let cron_name = cron_name.clone();
                    let cron_schedule = cron_schedule.clone();
                    let trace_store = trace_store.clone();

                    async move {
                        info!("Executing cron job: {}", cron_name);

                        let mut metadata: HashMap<String, String> = HashMap::new();
                        metadata.insert("cron_name".to_string(), cron_name.clone());
                        metadata.insert("schedule".to_string(), cron_schedule.clone());
                        let trace_id = trace_store
                            .start_trace(
                                cron_name.clone(),
                                crate::trace::TraceEntryType::Cron,
                                metadata,
                            )
                            .await;

                        let start = std::time::Instant::now();
                        let exec_result = executor.execute(&cron_name, serde_json::json!({})).await;
                        let duration_ms = start.elapsed().as_millis() as u64;

                        match exec_result {
                            Ok(result) => {
                                let mut triggered_events = Vec::new();

                                if result.success {
                                    info!("Cron job completed: {}", cron_name);

                                    for trigger in &triggers {
                                        let trigger_start = std::time::Instant::now();
                                        let payload = result
                                            .data
                                            .clone()
                                            .unwrap_or(serde_json::json!({}));
                                        let emit_res = event_bus.emit(trigger, payload).await;
                                        let trigger_duration =
                                            trigger_start.elapsed().as_millis() as u64;
                                        let trigger_timestamp = chrono::Utc::now().to_rfc3339();

                                        if let Err(e) = emit_res {
                                            tracing::error!(
                                                "Failed to emit event {}: {}",
                                                trigger,
                                                e
                                            );
                                        }

                                        triggered_events.push(crate::trace::TriggeredEventInfo {
                                            event_name: trigger.clone(),
                                            timestamp: trigger_timestamp,
                                            duration_ms: trigger_duration,
                                        });
                                    }
                                }

                                trace_store
                                    .add_step_with_triggers(
                                        &trace_id,
                                        cron_name.clone(),
                                        duration_ms
                                            .max(result.execution_time_ms),
                                        result.success,
                                        result.error.clone(),
                                        triggered_events,
                                    )
                                    .await;

                                let status = if result.success {
                                    crate::trace::TraceStatus::Success
                                } else {
                                    crate::trace::TraceStatus::Failed
                                };

                                trace_store
                                    .complete_trace(&trace_id, status, result.error.clone())
                                    .await;

                                if result.success {
                                    Ok(())
                                } else {
                                    Err(rohas_cron::CronError::ExecutionFailed(
                                        result
                                            .error
                                            .unwrap_or_else(|| "Unknown error".to_string()),
                                    ))
                                }
                            }
                            Err(e) => {
                                let err_msg = e.to_string();

                                trace_store
                                    .add_step(
                                        &trace_id,
                                        cron_name.clone(),
                                        duration_ms,
                                        false,
                                        Some(err_msg.clone()),
                                    )
                                    .await;

                                trace_store
                                    .complete_trace(
                                        &trace_id,
                                        crate::trace::TraceStatus::Failed,
                                        Some(err_msg.clone()),
                                    )
                                    .await;

                                Err(rohas_cron::CronError::ExecutionFailed(err_msg))
                            }
                        }
                    }
                })
                .await;
        }

        self.scheduler.start().await?;

        *initialized = true;
        info!("Engine initialized successfully");

        Ok(())
    }

    pub async fn start_server(&self) -> Result<()> {
        if !*self.initialized.read().await {
            return Err(EngineError::NotInitialized);
        }

        let addr = SocketAddr::from((
            self.config.server.host.parse::<std::net::IpAddr>().unwrap(),
            self.config.server.port,
        ));

        info!("Starting HTTP server on {}", addr);
        let arc_config = Arc::new(self.config.clone());
        let mut router = api::build_router(
            self.executor.clone(),
            self.schema.clone(),
            arc_config,
            self.event_bus.clone(),
            self.trace_store.clone(),
            self.tracing_log_store.clone(),
        );

        if self.config.server.enable_cors {
            router = router::with_cors(router);
        }

        let listener = tokio::net::TcpListener::bind(addr).await?;

        axum::serve(
            listener,
            router.into_make_service_with_connect_info::<SocketAddr>()
        )
            .await
            .map_err(|e| EngineError::Api(e.to_string()))?;

        Ok(())
    }

    pub async fn run(&self) -> Result<()> {
        self.initialize().await?;
        self.start_server().await?;
        Ok(())
    }

    pub async fn stats(&self) -> EngineStats {
        EngineStats {
            models_count: self.schema.models.len(),
            apis_count: self.schema.apis.len(),
            events_count: self.schema.events.len(),
            crons_count: self.schema.crons.len(),
            topics_count: self.adapter.list_topics().await.len(),
        }
    }

    pub async fn clear_handler_cache(&self) -> Result<()> {
        self.executor.clear_handler_cache().await?;
        Ok(())
    }

    pub fn tracing_log_store(&self) -> Arc<crate::tracing_log::TracingLogStore> {
        self.tracing_log_store.clone()
    }

    pub fn create_tracing_log_layer(&self) -> crate::tracing_log::TracingLogLayer {
        crate::tracing_log::TracingLogLayer::new(self.tracing_log_store.clone())
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EngineStats {
    pub models_count: usize,
    pub apis_count: usize,
    pub events_count: usize,
    pub crons_count: usize,
    pub topics_count: usize,
}
