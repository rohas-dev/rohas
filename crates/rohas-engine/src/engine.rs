use crate::api;
use crate::config::EngineConfig;
use crate::error::{EngineError, Result};
use crate::event::EventBus;
use crate::router;
use adapter_memory::MemoryAdapter;
use rohas_cron::{JobConfig, Scheduler};
use rohas_parser::{Parser, Schema};
use rohas_runtime::{Executor, RuntimeConfig};
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

        let adapter = Arc::new(MemoryAdapter::new(config.adapter.buffer_size));

        let event_bus = Arc::new(EventBus::new(
            adapter.clone(),
            executor.clone(),
            schema.clone(),
        ));

        let scheduler = Arc::new(Scheduler::new());

        Ok(Self {
            config,
            schema,
            executor,
            event_bus,
            scheduler,
            adapter,
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

        for cron in &self.schema.crons {
            let job_config = JobConfig::new(cron.name.clone(), cron.schedule.clone())
                .with_triggers(cron.triggers.clone());

            let job_id = self.scheduler.add_job(job_config).await?;
            info!("Registered cron job: {} ({})", cron.name, job_id);

            let cron_name = cron.name.clone();
            let executor = self.executor.clone();
            let event_bus = self.event_bus.clone();
            let triggers = cron.triggers.clone();

            self.scheduler
                .register_handler(&cron_name.clone(), move |_config| {
                    let executor = executor.clone();
                    let event_bus = event_bus.clone();
                    let triggers = triggers.clone();
                    let cron_name = cron_name.clone();

                    async move {
                        info!("Executing cron job: {}", cron_name);

                        match executor.execute(&cron_name, serde_json::json!({})).await {
                            Ok(result) => {
                                if result.success {
                                    info!("Cron job completed: {}", cron_name);

                                    for trigger in &triggers {
                                        if let Err(e) = event_bus
                                            .emit(
                                                trigger,
                                                result
                                                    .data
                                                    .clone()
                                                    .unwrap_or(serde_json::json!({})),
                                            )
                                            .await
                                        {
                                            tracing::error!(
                                                "Failed to emit event {}: {}",
                                                trigger,
                                                e
                                            );
                                        }
                                    }

                                    Ok(())
                                } else {
                                    Err(rohas_cron::CronError::ExecutionFailed(
                                        result.error.unwrap_or_else(|| "Unknown error".to_string()),
                                    ))
                                }
                            }
                            Err(e) => Err(rohas_cron::CronError::ExecutionFailed(e.to_string())),
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
        let mut router = api::build_router(self.executor.clone(), self.schema.clone(), arc_config);

        if self.config.server.enable_cors {
            router = router::with_cors(router);
        }

        let listener = tokio::net::TcpListener::bind(addr).await?;

        axum::serve(listener, router)
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
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EngineStats {
    pub models_count: usize,
    pub apis_count: usize,
    pub events_count: usize,
    pub crons_count: usize,
    pub topics_count: usize,
}
