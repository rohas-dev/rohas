use anyhow::Result;
use rohas_codegen::{generate, Language};
use rohas_parser::{Parser, Schema};
use std::fs;
use std::path::PathBuf;
use tracing::info;

pub async fn execute(
    schema_path: PathBuf,
    output_path: PathBuf,
    lang: Option<String>,
) -> Result<()> {
    info!("Generating code from schema: {}", schema_path.display());

    let language = match lang.as_deref() {
        Some("typescript") | Some("ts") => Language::TypeScript,
        Some("python") | Some("py") => Language::Python,
        None => Language::TypeScript,
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

fn parse_directory(dir: &PathBuf) -> Result<Schema> {
    let mut combined_schema = Schema::new();
    let mut file_count = 0;

    visit_roh_files(dir, &mut |path| {
        info!("Parsing: {}", path.display());
        match Parser::parse_file(path) {
            Ok(schema) => {
                // Merge schemas
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

fn visit_roh_files<F>(dir: &PathBuf, callback: &mut F) -> Result<()>
where
    F: FnMut(&PathBuf) -> Result<()>,
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
