mod rust_compiler;
mod ts_compiler;

use notify::RecursiveMode;
use notify_debouncer_full::{new_debouncer, DebounceEventResult};
use rohas_codegen::{self, Language as CodegenLanguage};
use rohas_engine::{config::Language as EngineLanguage, Engine, EngineConfig};
use rohas_parser::{Parser, Schema};
use rust_compiler::RustCompiler;
use tracing::debug;
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
    rust_compiler: Arc<RwLock<Option<RustCompiler>>>,
    rust_library: Arc<tokio::sync::Mutex<Option<libloading::Library>>>,
    last_loaded_dylib_hash: Arc<tokio::sync::Mutex<Option<[u8; 32]>>>,
}

impl DevServer {
    pub fn new(schema_path: PathBuf, config: EngineConfig, watch: bool) -> Self {
        Self {
            schema_path,
            config,
            watch,
            engine: Arc::new(RwLock::new(None)),
            ts_compiler: Arc::new(RwLock::new(None)),
            rust_compiler: Arc::new(RwLock::new(None)),
            rust_library: Arc::new(tokio::sync::Mutex::new(None)),
            last_loaded_dylib_hash: Arc::new(tokio::sync::Mutex::new(None)),
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

    fn is_rust_project(&self) -> bool {
        let project_root = self.get_project_root();
        RustCompiler::is_rust_project(&project_root)
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

        if self.is_rust_project() {
            info!("Detected Rust project");
            self.setup_rust_compiler().await?;
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

    async fn setup_rust_compiler(&self) -> anyhow::Result<()> {
        let project_root = self.get_project_root();
        info!(
            "Setting up Rust compiler in: {}",
            project_root.display()
        );

        let compiler = RustCompiler::new(project_root);

        compiler.compile()?;

        let mut rust_compiler = self.rust_compiler.write().await;
        *rust_compiler = Some(compiler);

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

        self.run_codegen(&schema)?;

        let engine = Engine::from_schema(schema, self.config.clone()).await?;

        let layer = engine.create_tracing_log_layer();
        if let Err(e) = rohas_engine::tracing_log::register_tracing_log_layer(layer) {
            warn!("Failed to register tracing log layer: {}", e);
        } else {
            info!("Tracing log layer registered");
        }

        engine.initialize().await?;

        {
            let mut rust_lib = self.rust_library.lock().await;
            *rust_lib = None;
        }

        if self.is_rust_project() {
            self.register_rust_handlers(&engine, true).await?;
        }

        let mut engine_lock = self.engine.write().await;
        *engine_lock = Some(engine);

        info!("Engine loaded and initialized");

        Ok(())
    }

    async fn register_rust_handlers(&self, engine: &Engine, should_build: bool) -> anyhow::Result<()> {
        let project_root = self.get_project_root();
        let executor = engine.executor();
        let rust_runtime = executor.rust_runtime().clone();

        info!("Registering Rust handlers via dylib loading...");

        let cargo_toml = project_root.join("Cargo.toml");
        if !cargo_toml.exists() {
            return Ok(());
        }

        let cargo_content = std::fs::read_to_string(&cargo_toml)?;
        let needs_dylib_config = !cargo_content.contains("crate-type") || !cargo_content.contains("dylib");

        if needs_dylib_config {
            let updated_content = if cargo_content.contains("[lib]") {
                if cargo_content.contains("crate-type") {
                    cargo_content.replace(
                        "crate-type = [",
                        "crate-type = [\"dylib\", "
                    )
                } else {
                    cargo_content.replace(
                        "[lib]",
                        "[lib]\ncrate-type = [\"dylib\", \"rlib\"]"
                    )
                }
            } else {
                format!("{}\n\n[lib]\ncrate-type = [\"dylib\", \"rlib\"]", cargo_content)
            };
            std::fs::write(&cargo_toml, updated_content)?;
            info!("Updated Cargo.toml to build as dylib");
        }

        let compiler = crate::rust_compiler::RustCompiler::new(project_root.clone());
        if should_build {
            info!("Building Rust project as dylib...");
            compiler.build_release().await?;
        } else {
            info!("Skipping build (already built)...");
        }

        let dylib_path = compiler.get_library_path_for_profile("release")?;

        if !dylib_path.exists() {
            let debug_dylib = compiler.get_library_path_for_profile("debug")?;

            if debug_dylib.exists() {
                return self.load_and_register_handlers(&debug_dylib, rust_runtime).await;
            } else {
                warn!("Rust dylib not found at: {} or {}", dylib_path.display(), debug_dylib.display());
                return Ok(());
            }
        }

        self.load_and_register_handlers_with_path(&dylib_path, rust_runtime, None).await
    }

    async fn load_and_register_handlers_with_path(
        &self,
        dylib_path: &std::path::Path,
        runtime: Arc<rohas_runtime::RustRuntime>,
        expected_hash: Option<[u8; 32]>,
    ) -> anyhow::Result<()> {
        if let Some(hash) = expected_hash {
            let mut last_hash = self.last_loaded_dylib_hash.lock().await;
            let prev_hash = *last_hash;
            drop(last_hash);
            
            let result = self.load_and_register_handlers(dylib_path, runtime).await;
            
            if result.is_ok() {
                let computed_hash: [u8; 32] = {
                    use sha2::{Sha256, Digest};
                    use std::io::Read;
                    let mut file = std::fs::File::open(dylib_path)?;
                    let mut hasher = Sha256::new();
                    let mut buffer = vec![0u8; 8192];
                    loop {
                        let bytes_read = file.read(&mut buffer)?;
                        if bytes_read == 0 {
                            break;
                        }
                        hasher.update(&buffer[..bytes_read]);
                    }
                    hasher.finalize().into()
                };
                
                if computed_hash != hash {
                    error!("ERROR: Computed dylib hash doesn't match expected hash!");
                    error!("This indicates the dylib file changed between rebuild and load.");
                    return Err(anyhow::anyhow!("Dylib hash mismatch"));
                }
            }
            
            result
        } else {
            self.load_and_register_handlers(dylib_path, runtime).await
        }
    }

    async fn load_and_register_handlers(
        &self,
        dylib_path: &std::path::Path,
        runtime: Arc<rohas_runtime::RustRuntime>,
    ) -> anyhow::Result<()> {
        use libloading::{Library, Symbol};
        use std::ffi::c_void;
        use std::sync::Arc;
        use sha2::{Sha256, Digest};
        use std::io::Read;

        info!("Loading Rust dylib from: {}", dylib_path.display());

        let dylib_hash: [u8; 32] = {
            let mut file = std::fs::File::open(dylib_path)?;
            let mut hasher = Sha256::new();
            let mut buffer = vec![0u8; 8192];
            loop {
                let bytes_read = file.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                hasher.update(&buffer[..bytes_read]);
            }
            hasher.finalize().into()
        };

        let last_hash = {
            let last_hash_lock = self.last_loaded_dylib_hash.lock().await;
            *last_hash_lock
        };

        if let Some(prev_hash) = last_hash {
            if dylib_hash == prev_hash {
                error!("ERROR: Dylib hash unchanged! The dylib file is identical to the last loaded version.");
                error!("This means either:");
                error!("  1. The code wasn't actually rebuilt (Cargo didn't detect changes)");
                error!("  2. macOS is caching the old dylib by path");
                error!("  3. The build produced identical output");
                error!("Hot reload will NOT work - the old code will still be running!");
                
                warn!("Attempting to force reload by waiting longer and using canonical path...");
                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                
                let canonical_path = dylib_path.canonicalize()?;
                info!("Using canonical path for dylib: {}", canonical_path.display());
                
            } else {
                let hash_prefix = u128::from_be_bytes(
                    dylib_hash[..16].try_into().unwrap_or([0; 16])
                );
                let prev_hash_prefix = u128::from_be_bytes(
                    prev_hash[..16].try_into().unwrap_or([0; 16])
                );
                info!("Dylib hash changed: 0x{:032x} -> 0x{:032x} (new dylib detected)", prev_hash_prefix, hash_prefix);
            }
        } else {
            let hash_prefix = u128::from_be_bytes(
                dylib_hash[..16].try_into().unwrap_or([0; 16])
            );
            info!("Loading dylib for the first time (hash: 0x{:032x})", hash_prefix);
        }

        let handler_count_before_load = runtime.handler_count().await;
        let handlers_before_load = runtime.list_handlers().await;
        info!("Handler state before loading dylib: count={}, handlers={:?}", handler_count_before_load, handlers_before_load);

        unsafe {
            let dylib_metadata = dylib_path.metadata()?;
            let dylib_size = dylib_metadata.len();
            let dylib_mtime = dylib_metadata.modified().ok();
            info!("Loading dylib: size={} bytes, mtime={:?}", dylib_size, dylib_mtime);

            let dylib_path_to_load = dylib_path.canonicalize().unwrap_or_else(|_| dylib_path.to_path_buf());
            info!("Loading dylib from canonical path: {}", dylib_path_to_load.display());

            let lib = Library::new(&dylib_path_to_load)?;
            info!("Dylib loaded successfully, registering handlers...");

            type SetRuntimeFn = unsafe extern "C" fn(*mut c_void) -> i32;
            let set_runtime: Symbol<SetRuntimeFn> = lib.get(b"rohas_set_runtime")?;

            let runtime_ptr = Arc::into_raw(runtime) as *mut c_void;

            let result = set_runtime(runtime_ptr);

            if result == 0 {
                let runtime: Arc<rohas_runtime::RustRuntime> = Arc::from_raw(runtime_ptr as *const rohas_runtime::RustRuntime);
                let runtime_clone = runtime.clone();
                std::mem::forget(runtime);

                let handler_count_after = runtime_clone.handler_count().await;
                let handlers_after = runtime_clone.list_handlers().await;
                info!("Rust handlers registered successfully: count={}, handlers={:?}", handler_count_after, handlers_after);

                if handler_count_after == 0 {
                    warn!("No handlers were registered from the dylib!");
                } else if handler_count_after == handler_count_before_load && handlers_after == handlers_before_load {
                    warn!("Handler count and list unchanged - handlers may not have been re-registered!");
                } else {
                    info!("Handlers successfully updated: {} -> {}", handler_count_before_load, handler_count_after);
                }

                let mut rust_lib = self.rust_library.lock().await;
                if rust_lib.is_some() {
                    warn!("Replacing existing dylib - this should only happen during hot reload");
                }
                *rust_lib = Some(lib);
                
                {
                    let mut last_hash = self.last_loaded_dylib_hash.lock().await;
                    *last_hash = Some(dylib_hash);
                }
                
                info!("Rust dylib kept in memory (hash stored for next reload verification)");
            } else {
                let _runtime: Arc<rohas_runtime::RustRuntime> = Arc::from_raw(runtime_ptr as *const rohas_runtime::RustRuntime);
                warn!("rohas_set_runtime returned error code: {}", result);
                return Err(anyhow::anyhow!("Failed to register Rust handlers: set_runtime returned {}", result));
            }

        }

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
                            let ext = path
                                .extension()
                                .and_then(|e| e.to_str())
                                .map(|s| s.to_ascii_lowercase());

                            if let Some(ext_str) = ext.as_deref() {
                                if ext_str == "ro" || ext_str == "roh" {
                                    if tx.blocking_send((path.clone(), ext_str.to_string())).is_err() {
                                        eprintln!("[File Watcher] Channel full, dropping event for: {}", path.display());
                                    }
                                }
                                else if ext_str == "ts" || ext_str == "tsx" || ext_str == "py" || ext_str == "rs" {
                                    eprintln!("[File Watcher] Detected {} file change: {}", ext_str, path.display());
                                    if tx.blocking_send((path.clone(), ext_str.to_string())).is_err() {
                                        eprintln!("[File Watcher] Channel full, dropping event for: {}", path.display());
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => eprintln!("[File Watcher] Watch error: {:?}", e),
            },
        )?;

        let schema_dir = self.schema_dir();

        debouncer.watch(&schema_dir, RecursiveMode::Recursive)?;

        info!("Watching for changes in: {}", schema_dir.display());

        let src_dir = self.get_project_root().join("src");
        if src_dir.exists() {
            debouncer.watch(&src_dir, RecursiveMode::Recursive)?;
            info!("Watching for handler changes in: {}", src_dir.display());
        }


        let _debouncer_guard = debouncer;

        let mut server_handle = {
            let engine = self.engine.clone();
            Some(tokio::spawn(async move {
                if let Some(eng) = engine.read().await.as_ref() {
                    if let Err(e) = eng.start_server().await {
                        error!("Server error: {}", e);
                    }
                }
            }))
        };


        let reloading = Arc::new(tokio::sync::RwLock::new(false));

        // Main event loop: respond to Ctrl+C and file changes.
        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    info!("Shutting down gracefully...");
                    if let Some(handle) = &server_handle {
                        handle.abort();
                    }
                    break;
                }
                maybe_msg = rx.recv() => {
                    let Some((path, ext)) = maybe_msg else {
                        error!("File watcher channel closed unexpectedly");
                        break;
                    };

                    info!("File change detected: {} (ext: {})", path.display(), ext);

                    let should_process = {
                        let is_reloading = reloading.read().await;
                        if *is_reloading {
                            info!("Ignoring file change during reload: {}", path.display());
                            false
                        } else {
                            true
                        }
                    };

                    if !should_process {
                        continue;
                    }

                    let ext = ext.to_ascii_lowercase();

                    if ext == "ro" || ext == "roh" {
                        warn!("Hot reload triggered - reloading engine (and restarting server)...");

                        {
                            let mut reloading_flag = reloading.write().await;
                            *reloading_flag = true;
                        }

                        if let Some(handle) = server_handle.take() {
                            info!("Stopping current HTTP server...");
                            handle.abort();
                            tokio::time::sleep(Duration::from_millis(500)).await;

                            let _ = tokio::time::timeout(Duration::from_millis(100), handle).await;
                        }

                        let reload_result = self.reload_engine().await;

                        {
                            let mut reloading_flag = reloading.write().await;
                            *reloading_flag = false;
                        }

                        match reload_result {
                            Ok(_) => {
                                info!("Engine reloaded successfully");

                                info!("Starting new HTTP server with updated schema...");
                                let engine = self.engine.clone();
                                let port = self.config.server.port;

                                let new_handle = tokio::spawn(async move {
                                    if let Some(eng) = engine.read().await.as_ref() {
                                        if let Err(e) = eng.start_server().await {
                                            error!("Server error: {}", e);
                                        }
                                    } else {
                                        error!("Engine not available when starting server");
                                    }
                                });

                                tokio::time::sleep(Duration::from_millis(200)).await;

                                if !new_handle.is_finished() {
                                    server_handle = Some(new_handle);
                                    info!("HTTP server restarted with new schema on port {}", port);
                                } else {
                                    match new_handle.await {
                                        Ok(_) => {
                                            error!("Server task completed unexpectedly - port may still be in use");
                                            warn!("Try waiting a moment and changing a schema file again, or restart the dev server");
                                        }
                                        Err(e) => {
                                            error!("Server task error: {}", e);
                                        }
                                    }
                                }

                                tokio::time::sleep(Duration::from_millis(100)).await;
                            }
                            Err(e) => {
                                error!("Failed to reload engine: {}", e);
                                warn!("Continuing to watch for changes...");
                            }
                        }
                    } else if ext == "ts" || ext == "tsx" {
                        let path_str = path.to_string_lossy();
                        let is_generated = path_str.contains("/generated/") || path_str.contains("\\generated\\");

                        if is_generated {
                            warn!("Generated file changed - clearing handler cache...");
                            if let Some(eng) = self.engine.read().await.as_ref() {
                                if let Err(e) = eng.clear_handler_cache().await {
                                    error!("Failed to clear handler cache: {}", e);
                                } else {
                                    info!("Handler cache cleared");
                                }
                            }
                        } else {
                            warn!("Handler file changed - reloading handler runtime...");

                            match self.reload_typescript_handler().await {
                                Ok(_) => {
                                    info!("Handler reloaded successfully");
                                }
                                Err(e) => {
                                    error!("Failed to reload handler: {}", e);
                                    warn!("Continuing to watch for changes...");
                                }
                            }
                        }
                    } else if ext == "py" {
                        let path_str = path.to_string_lossy();
                        let is_generated = path_str.contains("/generated/") || path_str.contains("\\generated\\");

                        if is_generated {
                            warn!("Generated file changed - clearing handler cache...");
                            if let Some(eng) = self.engine.read().await.as_ref() {
                                if let Err(e) = eng.clear_handler_cache().await {
                                    error!("Failed to clear handler cache: {}", e);
                                } else {
                                    info!("Handler cache cleared");
                                }
                            }
                        } else {
                            warn!("Python handler file changed - clearing cache...");
                            if let Some(eng) = self.engine.read().await.as_ref() {
                                if let Err(e) = eng.clear_handler_cache().await {
                                    error!("Failed to clear handler cache: {}", e);
                                } else {
                                    info!("Handler cache cleared");
                                }
                            }
                        }
                    } else if ext == "rs" {
                        let path_str = path.to_string_lossy();
                        let is_generated = path_str.contains("/generated/") || path_str.contains("\\generated\\");

                        if path_str.ends_with("/lib.rs") || path_str.ends_with("\\lib.rs") {
                            debug!("Ignoring lib.rs change (touched by build process): {}", path.display());
                            continue;
                        }
                        if path_str.ends_with("/generated/handlers.rs") || path_str.ends_with("\\generated\\handlers.rs") {
                            debug!("Ignoring handlers.rs change (touched by build process): {}", path.display());
                            continue;
                        }

                        info!("Rust file change detected: {} (generated: {})", path.display(), is_generated);

                        if is_generated {
                            warn!("Generated Rust file changed - recompiling...");
                            if let Err(e) = self.reload_rust_handler().await {
                                error!("Failed to recompile Rust handlers: {}", e);
                            } else {
                                info!("Successfully reloaded Rust handlers after generated file change");
                            }
                        } else {
                            warn!("Rust handler file changed - recompiling...");
                            if let Err(e) = self.reload_rust_handler_with_file(Some(path.as_path())).await {
                                error!("Failed to recompile Rust handlers: {}", e);
                                warn!("Continuing to watch for changes...");
                            } else {
                                info!("Successfully reloaded Rust handlers after handler file change");
                            }
                        }
                    }
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

    async fn reload_rust_handler(&self) -> anyhow::Result<()> {
        self.reload_rust_handler_with_file(None).await
    }

    async fn reload_rust_handler_with_file(&self, _changed_file: Option<&std::path::Path>) -> anyhow::Result<()> {
        {
            let rust_compiler = self.rust_compiler.read().await;
            if let Some(compiler) = rust_compiler.as_ref() {
                info!("Rebuilding Rust handlers as dylib...");

                let project_root = compiler.project_root().clone();
                let generated_handlers_rs = project_root.join("src").join("generated").join("handlers.rs");
                
                if generated_handlers_rs.exists() {
                    use std::io::Write;
                    if let Ok(mut file) = fs::OpenOptions::new().append(true).open(&generated_handlers_rs) {
                        let _ = file.write_all(b"\n");
                        drop(file);
                        info!("Touched generated handlers.rs to force Cargo rebuild: {}", generated_handlers_rs.display());
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    } else {
                        warn!("Failed to touch generated handlers.rs, but continuing with build");
                    }
                } else {
                    warn!("Generated handlers.rs not found at: {}, trying alternative approach", generated_handlers_rs.display());
                    let lib_rs = project_root.join("src").join("lib.rs");
                    if lib_rs.exists() {
                        use std::io::Write;
                        if let Ok(mut file) = fs::OpenOptions::new().append(true).open(&lib_rs) {
                            let _ = file.write_all(b"\n");
                            drop(file);
                            info!("Touched lib.rs as fallback to force Cargo rebuild");
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        }
                    }
                }

                let build_result = compiler.build_release().await;
                build_result?;

                let dylib_path = compiler.get_library_path_for_profile("release")?;
                if let Ok(metadata) = dylib_path.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        info!("Dylib last modified: {:?}", modified);
                    }
                }

                info!("Rust handlers rebuilt successfully");

                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            } else {
                warn!("Rust compiler not initialized, skipping rebuild");
                return Ok(());
            }
        }

        {
            let engine = self.engine.read().await;
            if let Some(eng) = engine.as_ref() {
                info!("Clearing Rust handler cache before reload...");
                let handler_count_before = eng.executor().rust_runtime().handler_count().await;
                let handlers_before = eng.executor().rust_runtime().list_handlers().await;
                info!("Handler count before clearing: {} ({:?})", handler_count_before, handlers_before);

                eng.executor().rust_runtime().clear_handlers().await;

                eng.clear_handler_cache().await?;

                let handler_count_after = eng.executor().rust_runtime().handler_count().await;
                let handlers_after = eng.executor().rust_runtime().list_handlers().await;
                info!("Handler count after clearing: {} ({:?})", handler_count_after, handlers_after);
                if handler_count_after > 0 {
                    warn!("Some handlers were not cleared! Attempting force clear...");
                    eng.executor().rust_runtime().clear_handlers().await;
                    let final_count = eng.executor().rust_runtime().handler_count().await;
                    if final_count > 0 {
                        error!("Handlers still not cleared after force clear! Count: {}", final_count);
                    } else {
                        info!("Handlers successfully cleared after force clear");
                    }
                }
            }
        }

        {
            let mut rust_lib = self.rust_library.lock().await;
            if rust_lib.is_some() {
                info!("Dropping old Rust dylib...");
                let old_lib = rust_lib.take();
                drop(old_lib);
                info!("Old Rust dylib dropped");
            } else {
                info!("No old Rust dylib to drop");
            }
        }

        info!("Waiting for OS to fully unload old dylib...");
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        info!("Proceeding to load new dylib...");

        let engine = self.engine.read().await;
        if let Some(eng) = engine.as_ref() {
            info!("Registering new Rust handlers...");

            let rust_compiler = self.rust_compiler.read().await;
            let (dylib_path, new_dylib_hash) = if let Some(compiler) = rust_compiler.as_ref() {
                let dylib_path = compiler.get_library_path_for_profile("release")?;
                if !dylib_path.exists() {
                    return Err(anyhow::anyhow!("Dylib not found at expected path: {}. Build may have failed.", dylib_path.display()));
                }
                
                use sha2::{Sha256, Digest};
                use std::io::Read;
                let hash: [u8; 32] = {
                    let mut file = std::fs::File::open(&dylib_path)?;
                    let mut hasher = Sha256::new();
                    let mut buffer = vec![0u8; 8192];
                    loop {
                        let bytes_read = file.read(&mut buffer)?;
                        if bytes_read == 0 {
                            break;
                        }
                        hasher.update(&buffer[..bytes_read]);
                    }
                    hasher.finalize().into()
                };
                
                let hash_prefix = u128::from_be_bytes(
                    hash[..16].try_into().unwrap_or([0; 16])
                );
                info!("New dylib hash after rebuild: 0x{:032x}", hash_prefix);
                
                let last_hash = {
                    let last_hash_lock = self.last_loaded_dylib_hash.lock().await;
                    *last_hash_lock
                };
                
                if let Some(prev_hash) = last_hash {
                    if hash == prev_hash {
                        error!("ERROR: New dylib hash is identical to previously loaded hash!");
                        error!("This means the rebuild didn't actually produce new code.");
                        error!("Hot reload will NOT work - the old code will still be running!");
                        return Err(anyhow::anyhow!("Dylib hash unchanged after rebuild - code was not actually rebuilt"));
                    } else {
                        let prev_hash_prefix = u128::from_be_bytes(
                            prev_hash[..16].try_into().unwrap_or([0; 16])
                        );
                        info!("Dylib hash changed after rebuild: 0x{:032x} -> 0x{:032x} (new code detected)", prev_hash_prefix, hash_prefix);
                    }
                }
                
                (dylib_path, hash)
            } else {
                return Err(anyhow::anyhow!("Rust compiler not available"));
            };

            #[cfg(target_os = "macos")]
            let dylib_path_to_use = {
                use std::time::{SystemTime, UNIX_EPOCH};
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos();
                
                let temp_dir = std::env::temp_dir();
                let temp_dylib_name = format!("librust_example_{}.dylib", timestamp);
                let temp_dylib_path = temp_dir.join(&temp_dylib_name);
                
                info!("Copying dylib to unique temp path to force fresh load: {}", temp_dylib_path.display());
                std::fs::copy(&dylib_path, &temp_dylib_path)?;
                info!("Dylib copied successfully");
                
                temp_dylib_path
            };
            
            #[cfg(not(target_os = "macos"))]
            let dylib_path_to_use = dylib_path.clone();

            self.load_and_register_handlers_with_path(&dylib_path_to_use, eng.executor().rust_runtime().clone(), Some(new_dylib_hash)).await?;
            
            #[cfg(target_os = "macos")]
            {
                let temp_path = dylib_path_to_use.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    if let Err(e) = std::fs::remove_file(&temp_path) {
                        warn!("Failed to clean up temporary dylib {}: {}", temp_path.display(), e);
                    } else {
                        info!("Cleaned up temporary dylib: {}", temp_path.display());
                    }
                });
            }

            let handler_count = eng.executor().rust_runtime().handler_count().await;
            let handlers = eng.executor().rust_runtime().list_handlers().await;
            info!("Handler count after registration: {}", handler_count);
            info!("Registered handlers: {:?}", handlers);

            if handler_count > 0 {
                info!("Handler registration completed - new closures should have been created");
                info!("Note: Function pointer addresses may be the same if macOS reuses memory, but closures should be new");
            }

            if handler_count == 0 {
                warn!("No handlers were registered! This indicates a problem.");
            } else {
                info!("Successfully reloaded {} Rust handler(s)", handler_count);
            }
        }

        Ok(())
    }

    fn run_codegen(&self, schema: &Schema) -> anyhow::Result<()> {
        let output_dir = self.config.project_root.join("src");

        let lang = match self.config.language {
            EngineLanguage::TypeScript => CodegenLanguage::TypeScript,
            EngineLanguage::Python => CodegenLanguage::Python,
            EngineLanguage::Rust => CodegenLanguage::Rust,
        };

        info!(
            "Running codegen for language {:?} into {}",
            lang,
            output_dir.display()
        );

        rohas_codegen::generate(schema, &output_dir, lang)?;

        info!("Codegen completed");

        Ok(())
    }
}

fn parse_directory(dir: &PathBuf) -> anyhow::Result<Schema> {
    let mut combined_schema = Schema::new();
    let mut file_count = 0;

    visit_ro_files(dir, &mut |path| {
        info!("Parsing: {}", path.display());
        match Parser::parse_file(path) {
            Ok(schema) => {
                combined_schema.models.extend(schema.models);
                combined_schema.types.extend(schema.types);
                combined_schema.inputs.extend(schema.inputs);
                combined_schema.apis.extend(schema.apis);
                combined_schema.events.extend(schema.events);
                combined_schema.crons.extend(schema.crons);
                combined_schema.websockets.extend(schema.websockets);
                file_count += 1;
                Ok(())
            }
            Err(e) => Err(anyhow::anyhow!("Failed to parse {}: {}", path.display(), e)),
        }
    })?;

    if file_count == 0 {
        anyhow::bail!("No .ro files found in {}", dir.display());
    }

    info!("Parsed {} schema files", file_count);

    combined_schema
        .validate()
        .map_err(|e| anyhow::anyhow!("Schema validation failed: {}", e))?;

    Ok(combined_schema)
}

fn visit_ro_files<F>(dir: &PathBuf, callback: &mut F) -> anyhow::Result<()>
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
            visit_ro_files(&path, callback)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("ro") {
            callback(&path)?;
        }
    }

    Ok(())
}
