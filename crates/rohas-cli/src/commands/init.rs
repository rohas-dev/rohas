use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use std::fs;
use std::path::Path;
use tracing::info;
use uuid::Uuid;

pub async fn execute(name: String, lang: String, _example: Option<String>) -> Result<()> {
    info!("Initializing new Rohas project: {}", name);

    let project_dir = Path::new(&name);

    if project_dir.exists() {
        anyhow::bail!("Directory '{}' already exists", name);
    }

    // Create project structure
    fs::create_dir_all(project_dir.join("schema/models"))?;
    fs::create_dir_all(project_dir.join("schema/api"))?;
    fs::create_dir_all(project_dir.join("schema/events"))?;
    fs::create_dir_all(project_dir.join("schema/cron"))?;
    fs::create_dir_all(project_dir.join("src/handlers/api"))?;
    fs::create_dir_all(project_dir.join("src/handlers/events"))?;
    fs::create_dir_all(project_dir.join("src/handlers/cron"))?;
    fs::create_dir_all(project_dir.join("config"))?;

    // Create example schema
    let user_model = r#"model User {
  id        Int      @id @auto
  name      String
  email     String   @unique
  createdAt DateTime @default(now)
}
"#;

    fs::write(project_dir.join("schema/models/user.ro"), user_model)?;

    let user_input = r#"input CreateUserInput {
  name: String
  email: String
}
"#;

    let user_api = r#"api CreateUser {
  method: POST
  path: "/users"
  body: CreateUserInput
  response: User
  triggers: [UserCreated]
}
"#;

    fs::write(
        project_dir.join("schema/api/user_api.ro"),
        format!("{}\n{}", user_input, user_api),
    )?;

    let user_event = r#"event UserCreated {
  payload: User
  handler: [send_welcome_email]
}
"#;

    fs::write(project_dir.join("schema/events/user_events.ro"), user_event)?;

    // Create rohas.toml
    let workbench_api_key = generate_workbench_api_key();
    let config = format!(
        r#"[project]
name = "{}"
version = "0.1.0"
language = "{}"

[server]
host = "127.0.0.1"
port = 3000
enable_cors = true

[adapter]
type = "memory"
buffer_size = 1000

[telemetry]
# Telemetry adapter type: rocksdb (default), prometheus, influxdb, timescaledb
type = "rocksdb"
# Path to telemetry storage (relative to project root or absolute)
path = ".rohas/telemetry"
# Retention period for traces in days (0 = keep forever)
retention_days = 30
# Maximum number of traces to keep in memory cache
max_cache_size = 1000
# Enable metrics collection
enable_metrics = true
# Enable logs collection
enable_logs = true
# Enable traces collection
enable_traces = true

[workbench]
api_key = "{}"
allowed_origins = []
"#,
        name, lang, workbench_api_key
    );

    fs::write(project_dir.join("config/rohas.toml"), config)?;

    // Create README
    let readme = format!(
        r#"# {}

Rohas project initialized with {} handlers.

## Getting Started

1. Generate code:
   ```bash
   rohas codegen
   ```

2. Start development server:
   ```bash
   rohas dev
   ```

   Or start with workbench UI:
   ```bash
   rohas dev --workbench
   ```

3. Validate schema:
   ```bash
   rohas validate
   ```

## Project Structure

- `schema/` - Schema definitions (.ro files)
- `src/handlers/` - Your handler implementations
- `config/` - Configuration files
"#,
        name, lang
    );

    fs::write(project_dir.join("README.md"), readme)?;

    info!("✓ Project '{}' created successfully!", name);
    info!("  Run 'cd {}' to enter the project directory", name);
    info!("  Run 'rohas codegen' to generate code");
    info!("  Run 'rohas dev' to start the development server");
    info!("  Run 'rohas dev --workbench' to start server with workbench UI");

    Ok(())
}

fn generate_workbench_api_key() -> String {
    general_purpose::STANDARD.encode(Uuid::new_v4().into_bytes())
}
