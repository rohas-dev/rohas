use anyhow::Result;
use rohas_dev_server::DevServer;
use rohas_engine::EngineConfig;
use std::path::PathBuf;
use tracing::info;

pub async fn execute(schema_path: PathBuf, _port: u16, watch: bool) -> Result<()> {
    info!("Starting development server...");

    let mut config = match EngineConfig::from_project_root() {
        Ok(config) => {
            info!("Loaded configuration from config/rohas.toml");
            config
        }
        Err(e) => {
            info!("Using default configuration ({})", e);
            EngineConfig::default()
        }
    };
    config.project_root = std::env::current_dir()?;
    let actual_path = if !schema_path.exists() && schema_path.ends_with("index.ro") {
        schema_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or(schema_path)
    } else {
        schema_path
    };

    let dev_server = DevServer::new(actual_path, config, watch);
    dev_server.run().await?;

    Ok(())
}
