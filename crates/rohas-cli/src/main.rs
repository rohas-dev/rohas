use clap::{Parser, Subcommand};
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
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
        } => {
            commands::dev::execute(schema, port, watch).await?;
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
