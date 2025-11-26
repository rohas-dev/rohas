use crate::utils::file_util::parse_directory;
use anyhow::Result;
use rohas_parser::Parser;
use std::path::PathBuf;
use tracing::info;

pub async fn execute(schema_path: PathBuf) -> Result<()> {
    info!("Validating schema: {}", schema_path.display());

    let schema = if schema_path.is_file() {
        Parser::parse_file(&schema_path)?
    } else if schema_path.is_dir() {
        parse_directory(&schema_path)?
    } else {
        anyhow::bail!("Schema path not found: {}", schema_path.display());
    };

    schema.validate()?;

    info!("✓ Schema validation passed!");
    info!("  - {} models", schema.models.len());
    info!("  - {} inputs", schema.inputs.len());
    info!("  - {} APIs", schema.apis.len());
    info!("  - {} events", schema.events.len());
    info!("  - {} cron jobs", schema.crons.len());

    Ok(())
}
