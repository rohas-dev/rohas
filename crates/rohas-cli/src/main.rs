use clap::{Parser, Subcommand};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use std::path::PathBuf;

mod commands;
mod utils;

#[derive(Parser)]
#[command(name = "rohas")]
#[command(about = "Rohas - Event-driven workflow orchestration framework", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init {
        name: String,

        #[arg(short, long, default_value = "typescript")]
        lang: String,

        #[arg(short, long)]
        example: Option<String>,
    },

    Codegen {
        #[arg(short, long, default_value = "schema")]
        schema: PathBuf,

        #[arg(short, long, default_value = "src")]
        output: PathBuf,

        #[arg(short, long)]
        lang: Option<String>,
    },

    Validate {
        #[arg(default_value = "schema")]
        schema: PathBuf,
    },

    Dev {
        #[arg(short, long, default_value = "schema")]
        schema: PathBuf,

        #[arg(short, long, default_value = "3000")]
        port: u16,

        #[arg(long, default_value = "true")]
        watch: bool,

        #[arg(long)]
        workbench: bool,

        #[arg(long)]
        workbench_dev: bool,
    },

    ListHandlers {
        #[arg(default_value = "schema")]
        schema: PathBuf,
    },

    ListEvents {
        #[arg(default_value = "schema")]
        schema: PathBuf,
    },

    Version,
}

use std::sync::Arc;
use tracing_subscriber::reload::Handle;

static TRACING_LOG_LAYER_HANDLE: std::sync::OnceLock<Arc<Handle<Option<rohas_engine::TracingLogLayer>, tracing_subscriber::Registry>>> = std::sync::OnceLock::new();

pub fn register_tracing_log_layer(layer: rohas_engine::TracingLogLayer) -> anyhow::Result<()> {
    if let Some(handle) = TRACING_LOG_LAYER_HANDLE.get() {
        handle.reload(Some(layer))?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    
    let (custom_layer, reload_handle) = tracing_subscriber::reload::Layer::new(None::<rohas_engine::TracingLogLayer>);
    
    let _ = TRACING_LOG_LAYER_HANDLE.set(Arc::new(reload_handle.clone()));
    rohas_engine::tracing_log::set_tracing_layer_handle(Arc::new(reload_handle));
    
    tracing_subscriber::registry()
        .with(custom_layer)
        .with(env_filter)
        .with(tracing_subscriber::fmt::Layer::default())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Init {
            name,
            lang,
            example,
        } => {
            commands::init::execute(name, lang, example).await?;
        }
        Commands::Codegen {
            schema,
            output,
            lang,
        } => {
            commands::codegen::execute(schema, output, lang).await?;
        }
        Commands::Validate { schema } => {
            commands::validate::execute(schema).await?;
        }
        Commands::Dev {
            schema,
            port,
            watch,
            workbench,
            workbench_dev,
        } => {
            commands::dev::execute(schema, port, watch, workbench, workbench_dev).await?;
        }
        Commands::ListHandlers { schema } => {
            commands::list::list_handlers(schema).await?;
        }
        Commands::ListEvents { schema } => {
            commands::list::list_events(schema).await?;
        }
        Commands::Version => {
            println!("rohas {}", env!("CARGO_PKG_VERSION"));
        }
    }

    Ok(())
}
