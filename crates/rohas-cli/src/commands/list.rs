use anyhow::Result;
use rohas_parser::Parser;
use std::path::PathBuf;

use crate::utils::file_util::parse_directory;

pub async fn list_handlers(schema_path: PathBuf) -> Result<()> {
    let schema = if schema_path.is_file() {
        Parser::parse_file(&schema_path)?
    } else if schema_path.is_dir() {
        parse_directory(&schema_path)?
    } else {
        anyhow::bail!("Schema path not found: {}", schema_path.display());
    };

    println!("API Handlers:");
    for api in &schema.apis {
        println!("  - {} ({} {})", api.name, api.method, api.path);
    }

    println!("\nEvent Handlers:");
    for event in &schema.events {
        println!("  - {} (handlers: {:?})", event.name, event.handlers);
    }

    println!("\nCron Jobs:");
    for cron in &schema.crons {
        println!("  - {} ({})", cron.name, cron.schedule);
    }

    Ok(())
}

pub async fn list_events(schema_path: PathBuf) -> Result<()> {
    let schema = if schema_path.is_file() {
        Parser::parse_file(&schema_path)?
    } else if schema_path.is_dir() {
        parse_directory(&schema_path)?
    } else {
        anyhow::bail!("Schema path not found: {}", schema_path.display());
    };

    println!("Events:");
    for event in &schema.events {
        println!("\n  {}", event.name);
        println!("    Payload: {}", event.payload);
        println!("    Handlers: {:?}", event.handlers);
        if !event.triggers.is_empty() {
            println!("    Triggers: {:?}", event.triggers);
        }
    }

    Ok(())
}
