use std::path::{Path, PathBuf};

use rohas_parser::{Parser, Schema};
use tracing::info;

pub fn find_config_file(start_dir: &Path) -> Option<PathBuf> {
    let mut current = if start_dir.is_absolute() {
        start_dir.to_path_buf()
    } else {
        std::env::current_dir()
            .ok()?
            .join(start_dir)
            .canonicalize()
            .ok()?
    }; 
    
    if current.is_file() {
        current = current.parent()?.to_path_buf();
    }

    if current.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&current) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let config_path = path.join("config").join("rohas.toml");
                    if config_path.exists() {
                        return Some(config_path);
                    }
                }
            }
        }
    } 

    loop {
        let config_path = current.join("config").join("rohas.toml");
        if config_path.exists() {
            return Some(config_path);
        }
 
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => break,
        }
    }

    None
}

pub fn parse_directory(dir: &PathBuf) -> anyhow::Result<Schema> {
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

pub fn visit_roh_files<F>(dir: &PathBuf, callback: &mut F) -> anyhow::Result<()>
where
    F: FnMut(&PathBuf) -> anyhow::Result<()>,
{
    if !dir.is_dir() {
        return Ok(());
    }

    let entries = std::fs::read_dir(dir)?;

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
