use anyhow::{Context, Result};
use rohas_orm::{Database, MigrationManager};
use rohas_parser::Parser;
use std::path::PathBuf;
use tracing::info;

pub async fn init(
    database_url: String,
    migrations_dir: Option<PathBuf>,
    migration_name: Option<String>,
    schema_path: Option<PathBuf>,
) -> Result<()> {
    info!("Initializing database: {}", database_url);

    let db = Database::connect(&database_url)
        .await
        .context("Failed to connect to database")?;

    let migrations_dir = migrations_dir.unwrap_or_else(|| PathBuf::from("migrations"));
    
    let manager = MigrationManager::new(migrations_dir.clone(), db.clone());
    manager.init().await.context("Failed to initialize migrations")?;

    info!("Database initialized successfully!");
    info!("Migration tracking table created: _rohas_migrations");

    if let Some(name) = migration_name {
        let schema_path = schema_path.unwrap_or_else(|| PathBuf::from("schema"));
        
        info!("Scanning schema at: {}", schema_path.display());
        info!("Creating migration: {}", name);

        let schema = if schema_path.is_file() {
            Parser::parse_file(&schema_path)
                .context("Failed to parse schema file")?
        } else if schema_path.is_dir() {
            let mut full_schema = rohas_parser::ast::Schema::new();
            fn scan_directory(dir: &std::path::Path, schema: &mut rohas_parser::ast::Schema) -> Result<()> {
                let entries = std::fs::read_dir(dir)
                    .context(format!("Failed to read directory: {:?}", dir))?;
                
                for entry in entries {
                    let entry = entry?;
                    let path = entry.path();
                    
                    if path.is_dir() {
                        scan_directory(&path, schema)?;
                    } else if path.extension().and_then(|s| s.to_str()) == Some("ro") {
                        let file_schema = Parser::parse_file(&path)
                            .context(format!("Failed to parse {:?}", path))?;
                        schema.models.extend(file_schema.models);
                    }
                }
                Ok(())
            }
            
            scan_directory(&schema_path, &mut full_schema)
                .context("Failed to scan schema directory")?;
            full_schema
        } else {
            anyhow::bail!("Schema path not found: {}", schema_path.display());
        };

        let (up_sql, down_sql) = manager.generate_migration_from_schema(
            &schema,
        )
        .await
        .context("Failed to generate migration SQL")?;

        let up_sql_trimmed = up_sql.trim();
        if up_sql_trimmed.is_empty() {
            info!("No schema changes detected. Migration not created.");
            return Ok(());
        }

        let mut migration = manager.create_migration(&name)
            .context("Failed to create migration file")?;

        migration.up_sql = up_sql;
        migration.down_sql = down_sql;

        let file_path = migrations_dir.join(format!("{}.sql", migration.name));
        let content = format!(
            "-- Up Migration\n{}\n\n-- Down Migration\n{}",
            migration.up_sql, migration.down_sql
        );
        std::fs::write(&file_path, content)
            .context("Failed to write migration file")?;

        info!("Migration file created: {}", file_path.display());

        info!("Applying migration...");
        manager.apply_migration(&migration).await
            .context("Failed to apply migration")?;

        info!("Migration applied successfully!");
    }

    Ok(())
}

pub async fn migrate(
    database_url: String,
    migrations_dir: Option<PathBuf>,
) -> Result<()> {
    info!("Applying pending migrations to database: {}", database_url);

    let db = Database::connect(&database_url)
        .await
        .context("Failed to connect to database")?;

    let migrations_dir = migrations_dir.unwrap_or_else(|| PathBuf::from("migrations"));
    let manager = MigrationManager::new(migrations_dir.clone(), db.clone());

    manager.init().await.ok();

    let applied = manager.get_applied_migrations().await
        .context("Failed to get applied migrations")?;

    let mut migration_files = Vec::new();
    if migrations_dir.exists() {
        let entries = std::fs::read_dir(&migrations_dir)
            .context("Failed to read migrations directory")?;
        
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("sql") {
                if let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) {
                    if !applied.contains(&file_name.to_string()) {
                        migration_files.push(path);
                    }
                }
            }
        }
    }

    migration_files.sort();

    if migration_files.is_empty() {
        info!("No pending migrations found.");
        return Ok(());
    }

    info!("Found {} pending migration(s)", migration_files.len());

    for file_path in migration_files {
        let content = std::fs::read_to_string(&file_path)
            .context(format!("Failed to read migration file: {:?}", file_path))?;

        // Parse migration file (format: -- Up Migration\n...\n\n-- Down Migration\n...)
        let parts: Vec<&str> = content.split("-- Down Migration").collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid migration file format: {:?}", file_path);
        }

        let up_sql = parts[0]
            .replace("-- Up Migration", "")
            .trim()
            .to_string();
        let down_sql = parts[1].trim().to_string();

        let migration_name = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let timestamp = migration_name
            .split('_')
            .next()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or_else(|| chrono::Utc::now().timestamp());

        let migration = rohas_orm::Migration {
            name: migration_name,
            timestamp,
            up_sql,
            down_sql,
        };

        info!("Applying migration: {}", migration.name);
        manager.apply_migration(&migration).await
            .context(format!("Failed to apply migration: {}", migration.name))?;
    }

    info!("All migrations applied successfully!");

    Ok(())
}

pub async fn deploy(
    database_url: String,
    migrations_dir: Option<PathBuf>,
) -> Result<()> {
    info!("Deploying migrations to database: {}", database_url);

    let db = Database::connect(&database_url)
        .await
        .context("Failed to connect to database")?;

    let migrations_dir = migrations_dir.unwrap_or_else(|| PathBuf::from("migrations"));
    let manager = MigrationManager::new(migrations_dir.clone(), db.clone());

    manager.init().await.ok();

    let applied = manager.get_applied_migrations().await
        .context("Failed to get applied migrations")?;

    let mut migration_files = Vec::new();
    if migrations_dir.exists() {
        let entries = std::fs::read_dir(&migrations_dir)
            .context("Failed to read migrations directory")?;
        
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("sql") {
                if let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) {
                    if !applied.contains(&file_name.to_string()) {
                        migration_files.push(path);
                    }
                }
            }
        }
    }

    migration_files.sort();

    if migration_files.is_empty() {
        info!("No pending migrations found.");
        return Ok(());
    }

    info!("Found {} pending migration(s)", migration_files.len());

    for file_path in migration_files {
        let content = std::fs::read_to_string(&file_path)
            .context(format!("Failed to read migration file: {:?}", file_path))?;

        // Parse migration file (format: -- Up Migration\n...\n\n-- Down Migration\n...)
        let parts: Vec<&str> = content.split("-- Down Migration").collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid migration file format: {:?}", file_path);
        }

        let up_sql = parts[0]
            .replace("-- Up Migration", "")
            .trim()
            .to_string();
        let down_sql = parts[1].trim().to_string();

        let migration_name = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let timestamp = migration_name
            .split('_')
            .next()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or_else(|| chrono::Utc::now().timestamp());

        let migration = rohas_orm::Migration {
            name: migration_name,
            timestamp,
            up_sql,
            down_sql,
        };

        info!("Applying migration: {}", migration.name);
        manager.apply_migration(&migration).await
            .context(format!("Failed to apply migration: {}", migration.name))?;
    }

    info!("All migrations deployed successfully!");

    Ok(())
}

/// Revert migrations (rollback the last N applied migrations)
pub async fn revert(
    database_url: String,
    migrations_dir: Option<PathBuf>,
    count: u32,
) -> Result<()> {
    info!("Reverting {} migration(s) from database: {}", count, database_url);

    let db = Database::connect(&database_url)
        .await
        .context("Failed to connect to database")?;

    let migrations_dir = migrations_dir.unwrap_or_else(|| PathBuf::from("migrations"));
    let manager = MigrationManager::new(migrations_dir.clone(), db.clone());

    manager.init().await.ok();

    let applied = manager.get_applied_migrations().await
        .context("Failed to get applied migrations")?;

    if applied.is_empty() {
        info!("No applied migrations found.");
        return Ok(());
    }

    let mut applied_migrations = Vec::new();
    if migrations_dir.exists() {
        let entries = std::fs::read_dir(&migrations_dir)
            .context("Failed to read migrations directory")?;
        
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("sql") {
                if let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) {
                    if applied.contains(&file_name.to_string()) {
                        applied_migrations.push(path);
                    }
                }
            }
        }
    }

    applied_migrations.sort();
    applied_migrations.reverse();

    let migrations_to_revert = applied_migrations.into_iter().take(count as usize).collect::<Vec<_>>();

    if migrations_to_revert.is_empty() {
        info!("No migrations to revert.");
        return Ok(());
    }

    info!("Reverting {} migration(s)", migrations_to_revert.len());

    for file_path in migrations_to_revert {
        let content = std::fs::read_to_string(&file_path)
            .context(format!("Failed to read migration file: {:?}", file_path))?;

        // Parse migration file (format: -- Up Migration\n...\n\n-- Down Migration\n...)
        let parts: Vec<&str> = content.split("-- Down Migration").collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid migration file format: {:?}", file_path);
        }

        let up_sql = parts[0]
            .replace("-- Up Migration", "")
            .trim()
            .to_string();
        let down_sql = parts[1].trim().to_string();

        let migration_name = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();


        let timestamp = migration_name
            .split('_')
            .next()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or_else(|| chrono::Utc::now().timestamp());

        let migration = rohas_orm::Migration {
            name: migration_name,
            timestamp,
            up_sql,
            down_sql,
        };

        info!("Reverting migration: {}", migration.name);
        manager.rollback_migration(&migration).await
            .context(format!("Failed to revert migration: {}", migration.name))?;
    }

    info!("Migration(s) reverted successfully!");

    Ok(())
}
