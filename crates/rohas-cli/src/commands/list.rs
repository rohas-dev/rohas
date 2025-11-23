use anyhow::Result;
use rohas_parser::Parser;
use std::path::PathBuf;

pub async fn list_handlers(schema_path: PathBuf) -> Result<()> {
    let schema = Parser::parse_file(&schema_path)?;

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
    let schema = Parser::parse_file(&schema_path)?;

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
