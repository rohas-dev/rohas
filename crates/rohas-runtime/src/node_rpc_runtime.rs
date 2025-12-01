use crate::error::{Result, RuntimeError};
use crate::handler::{HandlerContext, HandlerResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::{Mutex, Semaphore};
use tokio::time::timeout;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: RpcParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RpcParams {
    handler_path: String,
    context: HandlerContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RpcResponse {
    jsonrpc: String,
    id: u64,
    result: Option<HandlerResult>,
    error: Option<RpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RpcError {
    code: i32,
    message: String,
    data: Option<String>,
}

struct Worker {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    id: u64,
    last_used: Instant,
    request_id_counter: Arc<Mutex<u64>>,
}

impl Worker {
    async fn new(worker_id: u64, project_root: &Path) -> Result<Self> {
        let possible_paths = vec![
            std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(|p| p.join("worker.js"))),
            Some(project_root.join("worker.js")),
            Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/worker.js")),
            Some(PathBuf::from("crates/rohas-runtime/src/worker.js")),
        ];

        let worker_js = possible_paths
            .into_iter()
            .flatten()
            .find(|p| p.exists())
            .ok_or_else(|| {
                RuntimeError::ExecutionFailed(
                    "worker.js not found. Please ensure worker.js is in the project root or next to the executable.".into()
                )
            })?;

        info!("Using worker.js at: {:?}", worker_js);

        let mut cmd = Command::new("node");
        cmd.arg(&worker_js)
            .current_dir(project_root)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .env("NODE_ENV", "production");

        let mut child = cmd.spawn().map_err(|e| {
            RuntimeError::ExecutionFailed(format!("Failed to spawn Node.js worker: {}", e))
        })?;

        let stdin = child.stdin.take().ok_or_else(|| {
            RuntimeError::ExecutionFailed("Failed to get worker stdin".into())
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            RuntimeError::ExecutionFailed("Failed to get worker stdout".into())
        })?;

        let mut reader = BufReader::new(stdout);
        let mut line = String::new();

        match timeout(Duration::from_secs(5), reader.read_line(&mut line)).await {
            Ok(Ok(_)) => {
                let ready: serde_json::Value = serde_json::from_str(line.trim())
                    .map_err(|e| RuntimeError::ExecutionFailed(format!("Invalid ready signal: {}", e)))?;
                
                if ready.get("type") == Some(&serde_json::Value::String("ready".to_string())) {
                    info!("Worker {} ready", worker_id);
                } else {
                    return Err(RuntimeError::ExecutionFailed(
                        "Worker did not send ready signal".into()
                    ))?;
                }
            }
            _ => {
                return Err(RuntimeError::ExecutionFailed(
                    "Worker failed to start within timeout".into()
                ))?;
            }
        }

        Ok(Self {
            child,
            stdin,
            stdout: reader,
            id: worker_id,
            last_used: Instant::now(),
            request_id_counter: Arc::new(Mutex::new(0)),
        })
    }

    async fn execute(
        &mut self,
        handler_path: &Path,
        context: HandlerContext,
    ) -> Result<HandlerResult> {
        let mut counter = self.request_id_counter.lock().await;
        *counter += 1;
        let request_id = *counter;
        drop(counter);

        let request = RpcRequest {
            jsonrpc: "2.0".to_string(),
            id: request_id,
            method: "execute".to_string(),
            params: RpcParams {
                handler_path: handler_path.to_string_lossy().to_string(),
                context,
            },
        };

        let request_json = serde_json::to_string(&request)?;
        
        self.stdin
            .write_all(format!("{}\n", request_json).as_bytes())
            .await
            .map_err(|e| RuntimeError::ExecutionFailed(format!("Failed to write to worker: {}", e)))?;
        self.stdin.flush().await.map_err(|e| {
            RuntimeError::ExecutionFailed(format!("Failed to flush worker stdin: {}", e))
        })?;

        let mut line = String::new();
        self.stdout
            .read_line(&mut line)
            .await
            .map_err(|e| RuntimeError::ExecutionFailed(format!("Failed to read from worker: {}", e)))?;

        let response: RpcResponse = serde_json::from_str(line.trim()).map_err(|e| {
            RuntimeError::ExecutionFailed(format!("Failed to parse worker response: {}", e))
        })?;

        if let Some(error) = response.error {
            return Err(RuntimeError::ExecutionFailed(format!(
                "Worker error: {} (code: {})",
                error.message, error.code
            )))?;
        }

        self.last_used = Instant::now();
        response.result.ok_or_else(|| {
            RuntimeError::ExecutionFailed("Worker response missing result".into())
        })
    }

    fn is_alive(&mut self) -> bool {
        match self.child.try_wait() {
            Ok(Some(_)) => false,
            Ok(None) => true,
            Err(_) => false,
        }
    }

    fn kill(&mut self) {
        let _ = self.child.kill();
    }
}

pub struct WorkerPool {
    workers: Arc<Mutex<Vec<Option<Worker>>>>,
    semaphore: Arc<Semaphore>,
    project_root: PathBuf,
    max_workers: usize,
    worker_id_counter: Arc<Mutex<u64>>,
}

impl WorkerPool {
    pub fn new(project_root: PathBuf, max_workers: usize) -> Self {
        info!("Initializing worker pool with {} workers", max_workers);
        Self {
            workers: Arc::new(Mutex::new(Vec::new())),
            semaphore: Arc::new(Semaphore::new(max_workers)),
            project_root,
            max_workers,
            worker_id_counter: Arc::new(Mutex::new(0)),
        }
    }

    async fn get_or_create_worker(&self) -> Result<usize> {
        let _permit = self.semaphore.acquire().await.map_err(|e| {
            RuntimeError::ExecutionFailed(format!("Failed to acquire worker permit: {}", e))
        })?;

        let mut workers = self.workers.lock().await;

        for (idx, worker_opt) in workers.iter_mut().enumerate() {
            if let Some(worker) = worker_opt {
                if worker.is_alive() {
                    return Ok(idx);
                } else {
                    *worker_opt = None;
                }
            }
        }

        let mut counter = self.worker_id_counter.lock().await;
        *counter += 1;
        let worker_id = *counter;
        drop(counter);

        info!("Creating new worker {}", worker_id);
        let worker = Worker::new(worker_id, &self.project_root).await?;

        for (idx, worker_opt) in workers.iter_mut().enumerate() {
            if worker_opt.is_none() {
                *worker_opt = Some(worker);
                return Ok(idx);
            }
        }

        workers.push(Some(worker));
        Ok(workers.len() - 1)
    }

    async fn execute_with_worker(
        &self,
        handler_path: &Path,
        context: HandlerContext,
    ) -> Result<HandlerResult> {
        let worker_idx = self.get_or_create_worker().await?;
        let mut workers = self.workers.lock().await;

        if let Some(worker) = workers.get_mut(worker_idx).and_then(|w| w.as_mut()) {
            match worker.execute(handler_path, context).await {
                Ok(result) => Ok(result),
                Err(e) => {
                    warn!("Worker {} failed, will be replaced: {}", worker.id, e);
                    worker.kill();
                    workers[worker_idx] = None;
                    Err(e)
                }
            }
        } else {
            Err(RuntimeError::ExecutionFailed("Worker slot is empty".into()))
        }
    }
}

pub struct NodeRpcRuntime {
    worker_pool: Arc<WorkerPool>,
    project_root: PathBuf,
}

impl NodeRpcRuntime {
    pub fn new(project_root: PathBuf, max_workers: usize) -> Result<Self> {
        info!("Initializing Node.js RPC runtime");
        
        let worker_pool = Arc::new(WorkerPool::new(project_root.clone(), max_workers));
        
        Ok(Self {
            worker_pool,
            project_root,
        })
    }

    pub fn set_project_root(&mut self, root: PathBuf) {
        self.project_root = root;
    }

    fn resolve_handler_path(&self, handler_path: &Path) -> PathBuf {
        if let Some(ext) = handler_path.extension() {
            if ext == "ts" || ext == "tsx" {
                let mut compiled_path = if handler_path.is_absolute() {
                    let path_str = handler_path.to_string_lossy();
                    if let Some(src_pos) = path_str.find("/src/") {
                        let before_src = &path_str[..src_pos];
                        let after_src = &path_str[src_pos + 5..]; // +5 for "/src/"
                        PathBuf::from(format!("{}/.rohas/{}", before_src, after_src))
                    } else if path_str.contains("src/") {
                        PathBuf::from(path_str.replace("src/", ".rohas/"))
                    } else {
                        let relative = handler_path
                            .strip_prefix(&self.project_root)
                            .unwrap_or(handler_path);
                        let stripped = relative
                            .strip_prefix("src")
                            .unwrap_or(relative);
                        self.project_root.join(".rohas").join(stripped)
                    }
                } else {
                    let stripped = handler_path
                        .strip_prefix("src")
                        .unwrap_or(handler_path);
                    self.project_root.join(".rohas").join(stripped)
                };
                
                compiled_path.set_extension("js");

                if compiled_path.exists() {
                    debug!("Resolved to compiled path: {:?}", compiled_path);
                    return compiled_path;
                } else {
                    warn!(
                        "Compiled handler not found at {:?}. Please run 'npm run compile' or 'rspack build' to compile TypeScript files.",
                        compiled_path
                    );
                    return compiled_path;
                }
            }
        }

        if handler_path.is_absolute() {
            return handler_path.to_path_buf();
        }

        self.project_root.join(handler_path)
    }

    pub async fn execute_handler(
        &self,
        handler_path: &Path,
        context: HandlerContext,
    ) -> Result<HandlerResult> {
        let start = std::time::Instant::now();

        debug!("Executing JavaScript handler via RPC: {:?}", handler_path);

        let resolved_path = self.resolve_handler_path(handler_path);
        debug!("Resolved handler path: {:?}", resolved_path);

        let result = self
            .worker_pool
            .execute_with_worker(&resolved_path, context)
            .await?;

        // Process and emit logs as tracing events
        for log in &result.logs {
            let level = log.level.as_str();
            let message = format!("[{}] {}", log.handler, log.message);
            
            // Emit tracing event based on log level
            match level {
                "error" => {
                    tracing::error!(
                        handler = %log.handler,
                        timestamp = %log.timestamp,
                        ?log.fields,
                        "{}", message
                    );
                }
                "warn" => {
                    tracing::warn!(
                        handler = %log.handler,
                        timestamp = %log.timestamp,
                        ?log.fields,
                        "{}", message
                    );
                }
                "info" => {
                    tracing::info!(
                        handler = %log.handler,
                        timestamp = %log.timestamp,
                        ?log.fields,
                        "{}", message
                    );
                }
                "debug" => {
                    tracing::debug!(
                        handler = %log.handler,
                        timestamp = %log.timestamp,
                        ?log.fields,
                        "{}", message
                    );
                }
                "trace" => {
                    tracing::trace!(
                        handler = %log.handler,
                        timestamp = %log.timestamp,
                        ?log.fields,
                        "{}", message
                    );
                }
                _ => {
                    tracing::info!(
                        handler = %log.handler,
                        timestamp = %log.timestamp,
                        level = %log.level,
                        ?log.fields,
                        "{}", message
                    );
                }
            }
        }

        let execution_time_ms = start.elapsed().as_millis() as u64;
        Ok(HandlerResult {
            execution_time_ms,
            ..result
        })
    }

    pub async fn load_module(&self, _module_path: &Path) -> Result<()> {
        Ok(())
    }

    pub async fn reload_module(&self, _module_name: &str) -> Result<()> {
        Ok(())
    }

    pub async fn clear_cache(&self) -> Result<()> {
        info!("Cache clear requested (workers will reload modules on next use)");
        Ok(())
    }

    pub async fn get_loaded_modules(&self) -> Vec<String> {
        Vec::new()
    }
}

