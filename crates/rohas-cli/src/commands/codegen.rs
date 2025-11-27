use anyhow::Result;
use rohas_codegen::{generate, Language};
use rohas_engine::config::{EngineConfig, Language as EngineLanguage};
use rohas_parser::Parser;
use std::path::PathBuf;
use tracing::info;

use crate::utils::file_util::{find_config_file, parse_directory};

fn engine_language_to_codegen_language(lang: EngineLanguage) -> Language {
    match lang {
        EngineLanguage::TypeScript => Language::TypeScript,
        EngineLanguage::Python => Language::Python,
    }
}

pub async fn execute(
    schema_path: PathBuf,
    output_path: PathBuf,
    lang: Option<String>,
) -> Result<()> {
    info!("Generating code from schema: {}", schema_path.display());

    let language = match lang.as_deref() {
        Some("typescript") | Some("ts") => Language::TypeScript,
        Some("python") | Some("py") => Language::Python,
        None => match find_config_file(&std::env::current_dir().unwrap_or_default()) {
            Some(config_path) => match EngineConfig::from_file(&config_path) {
                Ok(config) => {
                    info!("Using language from config: {:?}", config.language);
                    engine_language_to_codegen_language(config.language)
                }
                Err(e) => {
                    info!(
                        "Could not parse config file, defaulting to TypeScript: {}",
                        e
                    );
                    Language::TypeScript
                }
            },
            None => {
                info!("Config file not found, defaulting to TypeScript");
                Language::TypeScript
            }
        },
        Some(other) => {
            anyhow::bail!("Unsupported language: {}", other);
        }
    };

    let schema = if schema_path.is_file() {
        Parser::parse_file(&schema_path)?
    } else if schema_path.is_dir() {
        parse_directory(&schema_path)?
    } else {
        anyhow::bail!("Schema path not found: {}", schema_path.display());
    };

    info!("Schema parsed successfully:");
    info!("  - {} models", schema.models.len());
    info!("  - {} inputs", schema.inputs.len());
    info!("  - {} APIs", schema.apis.len());
    info!("  - {} events", schema.events.len());
    info!("  - {} cron jobs", schema.crons.len());

    // Generate code
    generate(&schema, &output_path, language)?;

    info!("✓ Code generation completed successfully!");
    info!("  Output directory: {}", output_path.display());

    Ok(())
}
