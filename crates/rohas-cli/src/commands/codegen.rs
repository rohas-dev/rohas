use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use rohas_codegen::{generate, Language};
use rohas_engine::config::{EngineConfig, Language as EngineLanguage};
use rohas_parser::Parser;
use std::fs;
use std::path::{Path, PathBuf};
use toml_edit::{value, Array, DocumentMut, Item, Table};
use tracing::info;
use uuid::Uuid;

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

    let current_dir = std::env::current_dir().unwrap_or_default();
    let config_path = find_config_file(&current_dir);

    if let Some(config_path) = &config_path {
        ensure_workbench_config(config_path)?;
    }

    let language = match lang.as_deref() {
        Some("typescript") | Some("ts") => Language::TypeScript,
        Some("python") | Some("py") => Language::Python,
        None => match &config_path {
            Some(config_path) => match EngineConfig::from_file(config_path) {
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

fn ensure_workbench_config(config_path: &Path) -> Result<()> {
    let raw = fs::read_to_string(config_path)?;
    let mut doc: DocumentMut = raw.parse()?;
    let mut updated = false;

    if !doc.contains_key("workbench") {
        let mut table = Table::new();
        table.set_implicit(false);
        doc["workbench"] = Item::Table(table);
        updated = true;
    }

    let workbench_table = doc["workbench"]
        .as_table_mut()
        .expect("workbench to be a table");

    let needs_api_key = workbench_table
        .get("api_key")
        .and_then(|item| item.as_str())
        .map(|s| s.is_empty())
        .unwrap_or(true);

    if needs_api_key {
        workbench_table["api_key"] = value(generate_api_key());
        updated = true;
    }

    if !workbench_table.contains_key("allowed_origins") {
        let array = Array::new();
        workbench_table["allowed_origins"] = Item::Value(array.into());
        updated = true;
    }

    if updated {
        fs::write(config_path, doc.to_string())?;
        info!(
            "Updated workbench configuration in {}",
            config_path.display()
        );
    }

    Ok(())
}

fn generate_api_key() -> String {
    general_purpose::STANDARD.encode(Uuid::new_v4().into_bytes())
}
