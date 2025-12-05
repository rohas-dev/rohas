use anyhow::{Context, Result};
use rohas_dev_server::DevServer;
use rohas_engine::EngineConfig;
use std::path::PathBuf;
use tokio::signal;
use tracing::{error, info, warn};

pub async fn execute(
    schema_path: PathBuf,
    _port: u16,
    watch: bool,
    workbench: bool,
    workbench_dev: bool,
) -> Result<()> {
    info!("Starting development server...");

    let actual_path = if !schema_path.exists() && schema_path.ends_with("index.ro") {
        schema_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or(schema_path)
    } else {
        schema_path.clone()
    };

    let project_root = if actual_path.file_name()
        .and_then(|s| s.to_str())
        .map(|s| s == "schema")
        .unwrap_or(false)
    {
        actual_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
    } else {
        actual_path.clone()
    };

    let config_path = project_root.join("config").join("rohas.toml");
    let mut config = if config_path.exists() {
        match EngineConfig::from_file(&config_path) {
            Ok(mut cfg) => {
                cfg.project_root = project_root.clone();
                info!("Loaded configuration from {}", config_path.display());
                cfg
            }
            Err(e) => {
                info!("Failed to load config from {}: {}. Using defaults.", config_path.display(), e);
                let mut cfg = EngineConfig::default();
                cfg.project_root = project_root.clone();
                cfg
            }
        }
    } else {
        info!("Config file not found: {}. Using default configuration.", config_path.display());
        let mut cfg = EngineConfig::default();
        cfg.project_root = project_root.clone();
        cfg
    };

    let dev_server = DevServer::new(actual_path, config.clone(), watch);

    if workbench || workbench_dev {
        let workbench_path = find_or_init_workbench().await?;
        let workbench_path_for_task = workbench_path.clone();
        let workbench_config = config.clone();

        let workbench_handle = tokio::spawn(async move {
            start_workbench(workbench_path_for_task, workbench_dev, workbench_config).await
        });
        let workbench_abort = workbench_handle.abort_handle();

        let server_handle = tokio::spawn(async move { dev_server.run().await });
        let server_abort = server_handle.abort_handle();

        let ctrl_c = async {
            signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
            info!("Received Ctrl+C signal, shutting down...");
        };

        tokio::select! {
            _ = ctrl_c => {
                info!("Shutting down gracefully...");
                workbench_abort.abort();
                server_abort.abort();
            }
            result = server_handle => {
                if let Err(e) = result {
                    error!("Server error: {}", e);
                }
                workbench_abort.abort();
            }
            result = workbench_handle => {
                if let Err(e) = result {
                    error!("Workbench error: {}", e);
                }
                server_abort.abort();
            }
        }
    } else {
        dev_server.run().await?;
    }

    Ok(())
}

async fn find_or_init_workbench() -> Result<PathBuf> {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let mut current = exe_dir;
            for _ in 0..10 {
                let workbench_dir = current.join("workbench");
                if workbench_dir.exists() && workbench_dir.join("package.json").exists() {
                    info!(
                        "Found workbench source directory: {}",
                        workbench_dir.display()
                    );
                    return Ok(workbench_dir);
                }
                if let Some(parent) = current.parent() {
                    current = parent;
                } else {
                    break;
                }
            }
        }
    }

    info!("Workbench source not found. Attempting to initialize...");
    match init_workbench().await {
        Ok(workbench_dir) => {
            if workbench_dir.exists() && workbench_dir.join("package.json").exists() {
                info!("Workbench initialized successfully");
                return Ok(workbench_dir);
            }
        }
        Err(e) => {
            warn!("Could not initialize workbench automatically: {}", e);
        }
    }

    anyhow::bail!(
        "Workbench source not found and could not be initialized automatically.\n\
        \n\
        The workbench needs to be available before use. To set up workbench:\n\
        1. Clone the rohas repository: git clone https://github.com/rohas-dev/rohas.git\n\
        2. Install dependencies: cd rohas/workbench && pnpm install\n\
        \n\
        Note: The workbench will be built and started automatically using 'next build' and 'next start'"
    )
}

async fn start_workbench(
    workbench_path: PathBuf,
    dev_mode: bool,
    config: EngineConfig,
) -> Result<()> {
    use std::process::Stdio;
    use tokio::process::Command;

    info!("Starting workbench at: {}", workbench_path.display());

    if !workbench_path.exists() || !workbench_path.join("package.json").exists() {
        anyhow::bail!(
            "Workbench directory not found at {}. Please ensure the workbench source is available.",
            workbench_path.display()
        );
    }

    if !workbench_path.join("node_modules").exists() {
        warn!("node_modules not found. Attempting to install dependencies...");
        let install_status = Command::new("pnpm")
            .arg("install")
            .current_dir(&workbench_path)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await;

        if let Err(e) = install_status {
            warn!("Failed to run pnpm install: {}. Continuing anyway...", e);
        } else if let Ok(status) = install_status {
            if !status.success() {
                warn!("pnpm install failed. Continuing anyway...");
            }
        }
    }

    let workbench_port = 4401;
    let api_url = format!("http://{}:{}", config.server.host, config.server.port);
    let workbench_api_key = config.workbench.api_key.clone();

    if dev_mode {
        info!(
            "Starting workbench in development mode on port {}...",
            workbench_port
        );
        let mut dev_cmd = Command::new("pnpm")
            .arg("run")
            .arg("dev")
            .current_dir(&workbench_path)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .env("PORT", workbench_port.to_string())
            .env("NEXT_PUBLIC_ROHAS_API_URL", &api_url)
            .env("ROHAS_API_URL", &api_url)
            .env("NEXT_PUBLIC_ROHAS_WORKBENCH_API_KEY", &workbench_api_key)
            .env("ROHAS_WORKBENCH_API_KEY", &workbench_api_key)
            .spawn()
            .context("Failed to start Next.js dev server")?;

        info!(
            "Workbench dev server started on http://127.0.0.1:{}",
            workbench_port
        );

        let status = dev_cmd
            .wait()
            .await
            .context("Failed to wait for workbench dev server")?;

        if !status.success() {
            anyhow::bail!(
                "Workbench dev server exited with error: {:?}",
                status.code()
            );
        }
    } else {
        info!("Building workbench...");
        let build_status = Command::new("pnpm")
            .arg("run")
            .arg("build")
            .current_dir(&workbench_path)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .env("NEXT_PUBLIC_ROHAS_API_URL", &api_url)
            .env("ROHAS_API_URL", &api_url)
            .env("NEXT_PUBLIC_ROHAS_WORKBENCH_API_KEY", &workbench_api_key)
            .env("ROHAS_WORKBENCH_API_KEY", &workbench_api_key)
            .status()
            .await
            .context("Failed to run next build")?;

        if !build_status.success() {
            anyhow::bail!("Failed to build workbench");
        }

        info!("Workbench built successfully");

        info!("Starting workbench server on port {}...", workbench_port);

        let mut start_cmd = Command::new("pnpm")
            .arg("run")
            .arg("start")
            .current_dir(&workbench_path)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .env("PORT", workbench_port.to_string())
            .env("NEXT_PUBLIC_ROHAS_API_URL", &api_url)
            .env("ROHAS_API_URL", &api_url)
            .env("NEXT_PUBLIC_ROHAS_WORKBENCH_API_KEY", &workbench_api_key)
            .env("ROHAS_WORKBENCH_API_KEY", &workbench_api_key)
            .spawn()
            .context("Failed to start Next.js server")?;

        info!(
            "Workbench server started on http://127.0.0.1:{}",
            workbench_port
        );

        let status = start_cmd
            .wait()
            .await
            .context("Failed to wait for workbench server")?;

        if !status.success() {
            anyhow::bail!("Workbench server exited with error: {:?}", status.code());
        }
    }

    Ok(())
}

async fn init_workbench() -> Result<PathBuf> {
    let source_workbench = find_workbench_source()?;

    if let Some(source) = source_workbench {
        info!("Found workbench source at: {}", source.display());
        return Ok(source);
    }

    Err(anyhow::anyhow!("Could not find workbench source directory"))
}

fn find_workbench_source() -> Result<Option<PathBuf>> {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let mut current = exe_dir;
            for _ in 0..10 {
                let workbench_dir = current.join("workbench");
                if workbench_dir.exists() && workbench_dir.join("package.json").exists() {
                    return Ok(Some(workbench_dir));
                }
                if let Some(parent) = current.parent() {
                    current = parent;
                } else {
                    break;
                }
            }
        }
    }

    let mut common_paths: Vec<PathBuf> = Vec::new();

    // User-local install: ~/.rohas/workbench (Unix) or %USERPROFILE%\.rohas\workbench (Windows)
    if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        common_paths.push(PathBuf::from(home).join(".rohas").join("workbench"));
    }

    // Windows-specific: %LOCALAPPDATA%\rohas\workbench (when available)
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        common_paths.push(PathBuf::from(local_app_data).join("rohas").join("workbench"));
    }

    common_paths.push(PathBuf::from("/usr/local/lib/rohas/workbench"));
    common_paths.push(PathBuf::from("/opt/rohas/workbench"));

    for path in &common_paths {
        if path.exists() && path.join("package.json").exists() {
            return Ok(Some(path.clone()));
        }
    }

    Ok(None)
}
