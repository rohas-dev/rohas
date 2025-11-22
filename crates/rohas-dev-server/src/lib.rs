mod ts_compiler;

use notify::RecursiveMode;
use notify_debouncer_full::{new_debouncer, DebounceEventResult};
use rohas_engine::{Engine, EngineConfig};
use rohas_parser::{Parser, Schema};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use ts_compiler::TypeScriptCompiler;

pub struct DevServer {
    schema_path: PathBuf,
    config: EngineConfig,
    watch: bool,
    engine: Arc<RwLock<Option<Engine>>>,
    ts_compiler: Arc<RwLock<Option<TypeScriptCompiler>>>,
}

impl DevServer {
    pub fn new(schema_path: PathBuf, config: EngineConfig, watch: bool) -> Self {
        Self {
            schema_path,
            config,
            watch,
            engine: Arc::new(RwLock::new(None)),
            ts_compiler: Arc::new(RwLock::new(None)),
        }
    }

    fn get_project_root(&self) -> PathBuf {
        let absolute_schema_path = if self.schema_path.is_absolute() {
            self.schema_path.clone()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(&self.schema_path)
        };

        if absolute_schema_path.is_dir() {
            absolute_schema_path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        } else {
            absolute_schema_path
                .parent()
                .and_then(|p| p.parent())
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        }
    }

    fn is_typescript_project(&self) -> bool {
        let project_root = self.get_project_root();
        let package_json = project_root.join("package.json");
        let swcrc = project_root.join(".swcrc");
        package_json.exists() && swcrc.exists()
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        info!("Starting Rohas development server");
        info!("  Schema: {}", self.schema_path.display());
        info!("  Port: {}", self.config.server.port);
        info!("  Hot reload: {}", self.watch);

        if self.is_typescript_project() {
            info!("Detected TypeScript project");
            self.setup_typescript_compiler().await?;
        }

        self.reload_engine().await?;

        if self.watch {
            self.watch_files().await?;
        } else {
            let ctrl_c = async {
                tokio::signal::ctrl_c()
                    .await
                    .expect("Failed to install Ctrl+C handler");
                info!("Received Ctrl+C signal, shutting down...");
            };

            if let Some(engine) = self.engine.read().await.as_ref() {
                tokio::select! {
                    _ = ctrl_c => {
                        info!("Server stopped");
                    }
                    result = engine.run() => {
                        result?;
                    }
                }
            }
        }

        Ok(())
    }

    async fn setup_typescript_compiler(&self) -> anyhow::Result<()> {
        let project_root = self.get_project_root();
        info!(
            "Setting up TypeScript compiler in: {}",
            project_root.display()
        );

        let compiler = TypeScriptCompiler::new(project_root);

        compiler.compile()?;

        let mut ts_compiler = self.ts_compiler.write().await;
        *ts_compiler = Some(compiler);

        Ok(())
    }

    async fn reload_engine(&self) -> anyhow::Result<()> {
        info!("Loading engine...");

        let schema = if self.schema_path.is_file() {
            Parser::parse_file(&self.schema_path)?
        } else if self.schema_path.is_dir() {
            parse_directory(&self.schema_path)?
        } else {
            anyhow::bail!("Schema path not found: {}", self.schema_path.display());
        };

        let engine = Engine::from_schema(schema, self.config.clone()).await?;

        engine.initialize().await?;

        let mut engine_lock = self.engine.write().await;
        *engine_lock = Some(engine);

        info!("✓ Engine loaded and initialized");

        Ok(())
    }

    fn schema_dir(&self) -> PathBuf {
        if self.schema_path.is_dir() {
            self.schema_path.clone()
        } else {
            self.schema_path
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .to_path_buf()
        }
    }

    async fn watch_files(&self) -> anyhow::Result<()> {
        info!("Starting file watcher...");

        let (tx, mut rx) = tokio::sync::mpsc::channel(100);

        let mut debouncer = new_debouncer(
            Duration::from_millis(500),
            None,
            move |result: DebounceEventResult| match result {
                Ok(events) => {
                    for event in events {
                        if let Some(path) = event.paths.first() {
                            let ext = path.extension().and_then(|e| e.to_str());
                            if ext == Some("roh")
                                || ext == Some("ts")
                                || ext == Some("tsx")
                                || ext == Some("py")
                            {
                                let _ =
                                    tx.blocking_send((path.clone(), ext.unwrap_or("").to_string()));
                            }
                        }
                    }
                }
                Err(e) => error!("Watch error: {:?}", e),
            },
        )?;

        let schema_dir = self.schema_dir();

        debouncer.watch(&schema_dir, RecursiveMode::Recursive)?;

        info!("Watching for changes in: {}", schema_dir.display());

        if self.is_typescript_project() {
            let src_dir = self.get_project_root().join("src");
            if src_dir.exists() {
                debouncer.watch(&src_dir, RecursiveMode::Recursive)?;
                info!("Watching for TypeScript changes in: {}", src_dir.display());
            }
        }

        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
            info!("Received Ctrl+C signal, shutting down...");
        };

        let engine = self.engine.clone();
        let server_handle = tokio::spawn(async move {
            if let Some(eng) = engine.read().await.as_ref() {
                if let Err(e) = eng.start_server().await {
                    error!("Server error: {}", e);
                }
            }
        });

        tokio::select! {
            _ = ctrl_c => {
                info!("Shutting down gracefully...");
                server_handle.abort();
                return Ok(());
            }
            result = self.handle_file_changes(&mut rx) => {
                result?;
            }
        }

        Ok(())
    }

    async fn handle_file_changes(
        &self,
        rx: &mut tokio::sync::mpsc::Receiver<(PathBuf, String)>,
    ) -> anyhow::Result<()> {
        while let Some((path, ext)) = rx.recv().await {
            info!("File changed: {}", path.display());

            if ext == "roh" {
                warn!("Hot reload triggered - reloading engine...");

                if let Err(e) = self.reload_engine().await {
                    error!("Failed to reload engine: {}", e);
                } else {
                    info!("✓ Engine reloaded successfully");
                }
            } else if ext == "ts" || ext == "tsx" {
                warn!("Handler file changed - recompiling...");

                if let Err(e) = self.reload_typescript_handler().await {
                    error!("Failed to reload handler: {}", e);
                } else {
                    info!("✓ Handler reloaded successfully");
                }
            }
        }

        Ok(())
    }

    async fn reload_typescript_handler(&self) -> anyhow::Result<()> {
        {
            let ts_compiler = self.ts_compiler.read().await;
            if let Some(compiler) = ts_compiler.as_ref() {
                compiler.compile()?;
            }
        }

        {
            let engine = self.engine.read().await;
            if let Some(eng) = engine.as_ref() {
                eng.clear_handler_cache().await?;
            }
        }

        Ok(())
    }
}

fn parse_directory(dir: &PathBuf) -> anyhow::Result<Schema> {
    let mut combined_schema = Schema::new();
    let mut file_count = 0;

    visit_roh_files(dir, &mut |path| {
        info!("Parsing: {}", path.display());
        match Parser::parse_file(path) {
            Ok(schema) => {
                combined_schema.models.extend(schema.models);
                combined_schema.inputs.extend(schema.inputs);
                combined_schema.apis.extend(schema.apis);
                combined_schema.events.extend(schema.events);
                combined_schema.crons.extend(schema.crons);
                file_count += 1;
                Ok(())
            }
            Err(e) => Err(anyhow::anyhow!("Failed to parse {}: {}", path.display(), e)),
        }
    })?;

    if file_count == 0 {
        anyhow::bail!("No .roh files found in {}", dir.display());
    }

    info!("Parsed {} schema files", file_count);

    combined_schema
        .validate()
        .map_err(|e| anyhow::anyhow!("Schema validation failed: {}", e))?;

    Ok(combined_schema)
}

fn visit_roh_files<F>(dir: &PathBuf, callback: &mut F) -> anyhow::Result<()>
where
    F: FnMut(&PathBuf) -> anyhow::Result<()>,
{
    if !dir.is_dir() {
        return Ok(());
    }

    let entries = fs::read_dir(dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            visit_roh_files(&path, callback)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("roh") {
            callback(&path)?;
        }
    }

    Ok(())
}
