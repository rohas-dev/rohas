use crate::connection::{Database, DatabaseType};
use crate::error::{Error, Result};
use rohas_parser::ast::{Field, FieldType, Model, Schema};
use std::fs;
use std::path::PathBuf;
use chrono::Utc;
use sqlx::Row;

#[derive(Debug, Clone)]
pub struct Migration {
    pub name: String,
    pub timestamp: i64,
    pub up_sql: String,
    pub down_sql: String,
}

pub struct MigrationManager {
    migrations_dir: PathBuf,
    pub(crate) database: Database,
}

impl MigrationManager {
    pub fn new(migrations_dir: PathBuf, database: Database) -> Self {
        Self {
            migrations_dir,
            database,
        }
    }

    pub async fn init(&self) -> Result<()> {
        let create_table_sql = match self.database.database_type() {
            DatabaseType::Postgres => r#"
                CREATE TABLE IF NOT EXISTS _rohas_migrations (
                    id SERIAL PRIMARY KEY,
                    name VARCHAR(255) NOT NULL UNIQUE,
                    timestamp BIGINT NOT NULL,
                    applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                );
            "#,
            DatabaseType::Sqlite => r#"
                CREATE TABLE IF NOT EXISTS _rohas_migrations (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL UNIQUE,
                    timestamp INTEGER NOT NULL,
                    applied_at DATETIME DEFAULT CURRENT_TIMESTAMP
                );
            "#,
            DatabaseType::MySql => r#"
                CREATE TABLE IF NOT EXISTS _rohas_migrations (
                    id INT AUTO_INCREMENT PRIMARY KEY,
                    name VARCHAR(255) NOT NULL UNIQUE,
                    timestamp BIGINT NOT NULL,
                    applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                );
            "#,
        };

        self.database.execute(create_table_sql.trim()).await?;
        Ok(())
    }

    pub async fn generate_migration_from_schema(
        &self,
        schema: &Schema,
    ) -> Result<(String, String)> {
        let database_type = self.database.database_type();
        let mut up_sql = String::new();
        let mut down_sql = String::new();

        let existing_tables = self.get_existing_tables().await?;

        for model in &schema.models {
            let table_name = Self::get_table_name(model);
            
            if existing_tables.contains(&table_name) {
                let (alter_up, alter_down) = self.generate_alter_table(model, &table_name, &database_type, Some(schema)).await?;
                if !alter_up.is_empty() {
                    up_sql.push_str(&alter_up);
                    up_sql.push_str("\n\n");
                    
                    if !alter_down.is_empty() {
                        down_sql.push_str(&alter_down);
                        down_sql.push_str("\n\n");
                    }
                }
            } else {
                let create_table = Self::generate_create_table_with_schema(model, &database_type, Some(schema))?;
                up_sql.push_str(&create_table);
                up_sql.push_str("\n\n");

                down_sql.push_str(&format!("DROP TABLE IF EXISTS {};\n", table_name));
            }
        }

        let join_tables = Self::generate_join_tables(schema, &database_type)?;
        up_sql.push_str(&join_tables.up);
        down_sql.push_str(&join_tables.down);

        Ok((up_sql, down_sql))
    }

    async fn get_existing_tables(&self) -> Result<Vec<String>> {
        use std::collections::HashMap;
        let query = match self.database.database_type() {
            DatabaseType::Postgres => "SELECT tablename FROM pg_tables WHERE schemaname = 'public'",
            DatabaseType::Sqlite => "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
            DatabaseType::MySql => "SELECT table_name FROM information_schema.tables WHERE table_schema = DATABASE()",
        };

        use crate::query::Query;
        use crate::query_builder::QueryBuilder;
        
        let mut tables = Vec::new();
        
        match self.database.database_type() {
            DatabaseType::Sqlite => {
                let pool = self.database.sqlite_pool()?;
                let query = "SELECT name FROM sqlite_master WHERE type='table'";
                let rows = sqlx::query(query)
                    .fetch_all(pool)
                    .await
                    .map_err(|e| Error::Connection(format!("Failed to get tables: {}", e)))?;
                
                for row in rows {
                    if let Ok(name) = row.try_get::<String, _>("name") {
                        if name != "_rohas_migrations" && !name.starts_with("sqlite_") {
                            tables.push(name);
                        }
                    }
                }
            }
            DatabaseType::Postgres => {
                let results = QueryBuilder::select_all()
                    .from("pg_tables")
                    .where_eq("schemaname", "public")
                    .execute(&self.database)
                    .await?;
                
                for row in results {
                    if let Some(table_name) = row.get("tablename").and_then(|v| v.as_str()) {
                        tables.push(table_name.to_string());
                    }
                }
            }
            DatabaseType::MySql => {
                let results = QueryBuilder::select_all()
                    .from("information_schema.tables")
                    .execute(&self.database)
                    .await?;
                
                for row in results {
                    if let Some(table_name) = row.get("table_name").and_then(|v| v.as_str()) {
                        tables.push(table_name.to_string());
                    }
                }
            }
        }

        Ok(tables)
    }

    async fn generate_alter_table(
        &self,
        model: &Model,
        table_name: &str,
        db_type: &DatabaseType,
        schema: Option<&Schema>,
    ) -> Result<(String, String)> {
        let existing_columns = self.get_table_columns(table_name).await?;
        
        let mut expected_columns = std::collections::HashMap::new();
        for field in &model.fields {
            if let FieldType::Custom(_) = &field.field_type {
                if field.attributes.iter().any(|attr| attr.name == "relation") {
                    continue;
                }
            }
            
            if let FieldType::Array(inner) = &field.field_type {
                if let FieldType::Custom(_) = inner.as_ref() {
                    continue;
                }
            }
            
            let column_def = Self::field_to_column(field, db_type)?;
            let column_name = field.name.clone();
            expected_columns.insert(column_name.clone(), column_def.column.clone());
        }

        let mut alter_sql = String::new();
        let mut down_sql = String::new();

        let removed_columns: Vec<(String, String)> = existing_columns.iter()
            .filter(|(name, _)| !expected_columns.contains_key(*name))
            .map(|(name, typ)| (name.clone(), typ.clone()))
            .collect();

        let new_columns: Vec<(String, String, String)> = expected_columns.iter()
            .filter(|(name, _)| !existing_columns.contains_key(*name))
            .map(|(name, def)| {
                let parts: Vec<&str> = def.split_whitespace().collect();
                let col_type = if parts.len() >= 2 {
                    parts[1].to_uppercase()
                } else {
                    "TEXT".to_string()
                };
                (name.clone(), def.clone(), col_type)
            })
            .collect();

        let mut handled_removed = std::collections::HashSet::new();
        let mut handled_new = std::collections::HashSet::new();

        for (old_name, old_type) in &removed_columns {
            for (new_name, new_def, new_type) in &new_columns {
                if handled_new.contains(new_name) {
                    continue;
                }
                if old_type.to_uppercase() == *new_type {
                    match db_type {
                        DatabaseType::Postgres => {
                            alter_sql.push_str(&format!("ALTER TABLE {} RENAME COLUMN {} TO {};\n", table_name, old_name, new_name));
                            down_sql.push_str(&format!("ALTER TABLE {} RENAME COLUMN {} TO {};\n", table_name, new_name, old_name));
                        }
                        DatabaseType::Sqlite => {
                            // SQLite supports RENAME COLUMN (since version 3.25.0)
                            alter_sql.push_str(&format!("ALTER TABLE {} RENAME COLUMN {} TO {};\n", table_name, old_name, new_name));
                            down_sql.push_str(&format!("ALTER TABLE {} RENAME COLUMN {} TO {};\n", table_name, new_name, old_name));
                        }
                        DatabaseType::MySql => {
                            let parts: Vec<&str> = new_def.split_whitespace().collect();
                            if parts.len() >= 2 {
                                let type_and_constraints = parts[1..].join(" ");
                                alter_sql.push_str(&format!("ALTER TABLE {} CHANGE COLUMN {} {} {};\n", table_name, old_name, new_name, type_and_constraints));
                                down_sql.push_str(&format!("ALTER TABLE {} CHANGE COLUMN {} {} {};\n", table_name, new_name, old_name, type_and_constraints));
                            }
                        }
                    }
                    handled_removed.insert(old_name.clone());
                    handled_new.insert(new_name.clone());
                    break;
                }
            }
        }

        for (column_name, column_def, _) in &new_columns {
            if !handled_new.contains(column_name) {
                match db_type {
                    DatabaseType::Postgres | DatabaseType::MySql => {
                        alter_sql.push_str(&format!("ALTER TABLE {} ADD COLUMN {};\n", table_name, column_def));
                        down_sql.push_str(&format!("ALTER TABLE {} DROP COLUMN {};\n", table_name, column_name));
                    }
                    DatabaseType::Sqlite => {
                        let parts: Vec<&str> = column_def.split_whitespace().collect();
                        if parts.len() >= 2 {
                            let type_and_constraints = parts[1..].join(" ");
                            let mut alter_stmt = format!("ALTER TABLE {} ADD COLUMN {} {}", table_name, column_name, type_and_constraints);
                            alter_sql.push_str(&format!("{};\n", alter_stmt));
                        } else {
                            alter_sql.push_str(&format!("ALTER TABLE {} ADD COLUMN {} TEXT;\n", table_name, column_name));
                        }
                        down_sql.push_str(&format!("ALTER TABLE {} DROP COLUMN {};\n", table_name, column_name));
                    }
                }
            }
        }

        for (column_name, _) in &removed_columns {
            if !handled_removed.contains(column_name) {
                match db_type {
                    DatabaseType::Postgres | DatabaseType::MySql => {
                        alter_sql.push_str(&format!("ALTER TABLE {} DROP COLUMN {};\n", table_name, column_name));
                        down_sql.push_str(&format!("-- Cannot automatically recreate dropped column {}\n", column_name));
                    }
                    DatabaseType::Sqlite => {
                        alter_sql.push_str(&format!("-- SQLite DROP COLUMN requires recreating the table\n"));
                        alter_sql.push_str(&format!("-- ALTER TABLE {} DROP COLUMN {};\n", table_name, column_name));
                        down_sql.push_str(&format!("-- Cannot automatically recreate dropped column {}\n", column_name));
                    }
                }
            }
        }

        Ok((alter_sql, down_sql))
    }

    async fn get_table_columns(&self, table_name: &str) -> Result<std::collections::HashMap<String, String>> {
        use std::collections::HashMap;
        let mut columns = HashMap::new();

        match self.database.database_type() {
            DatabaseType::Sqlite => {
                let pool = self.database.sqlite_pool()?;
                let query = format!("PRAGMA table_info({})", table_name);
                
                let rows = sqlx::query(&query)
                    .fetch_all(pool)
                    .await
                    .map_err(|e| Error::Connection(format!("Failed to get table info: {}", e)))?;
                
                for row in rows {
                    if let Ok(name) = row.try_get::<String, _>("name") {
                        if let Ok(typ) = row.try_get::<String, _>("type") {
                            columns.insert(name, typ.to_uppercase());
                        }
                    }
                }
            }
            DatabaseType::Postgres => {
                let query = format!(
                    "SELECT column_name, data_type FROM information_schema.columns WHERE table_name = '{}'",
                    table_name
                );
                use crate::query::Query;
                use crate::query_builder::QueryBuilder;
                
                let results = QueryBuilder::select_all()
                    .from("information_schema.columns")
                    .where_eq("table_name", table_name)
                    .execute(&self.database)
                    .await?;
                
                for row in results {
                    if let Some(col_name) = row.get("column_name").and_then(|v| v.as_str()) {
                        if let Some(data_type) = row.get("data_type").and_then(|v| v.as_str()) {
                            columns.insert(col_name.to_string(), data_type.to_string());
                        }
                    }
                }
            }
            DatabaseType::MySql => {
                let query = format!(
                    "SELECT column_name, data_type FROM information_schema.columns WHERE table_name = '{}' AND table_schema = DATABASE()",
                    table_name
                );
                use crate::query::Query;
                use crate::query_builder::QueryBuilder;
                
                let results = QueryBuilder::select_all()
                    .from("information_schema.columns")
                    .where_eq("table_name", table_name)
                    .execute(&self.database)
                    .await?;
                
                for row in results {
                    if let Some(col_name) = row.get("column_name").and_then(|v| v.as_str()) {
                        if let Some(data_type) = row.get("data_type").and_then(|v| v.as_str()) {
                            columns.insert(col_name.to_string(), data_type.to_string());
                        }
                    }
                }
            }
        }

        Ok(columns)
    }

    fn generate_create_table_with_schema(
        model: &Model,
        db_type: &DatabaseType,
        schema: Option<&Schema>,
    ) -> Result<String> {
        let table_name = Self::get_table_name(model);
        let mut sql = format!("CREATE TABLE IF NOT EXISTS {} (\n", table_name);

        let mut columns = Vec::new();
        let mut primary_key = None;
        let mut unique_constraints = Vec::new();
        let mut indexes = Vec::new();

        let mut primary_key_in_column = false;
        for field in &model.fields {
            if let FieldType::Custom(_) = &field.field_type {
                if field.attributes.iter().any(|attr| attr.name == "relation") {
                    continue;
                }
            }
            
            if let FieldType::Array(inner) = &field.field_type {
                if let FieldType::Custom(_) = inner.as_ref() {
                    continue;
                }
            }
            
            let column_def = Self::field_to_column(field, db_type)?;
            columns.push(format!("    {}", column_def.column));

            if column_def.is_primary {
                if column_def.column.contains("PRIMARY KEY") {
                    primary_key_in_column = true;
                } else {
                    primary_key = Some(field.name.clone());
                }
            }

            if column_def.is_unique {
                unique_constraints.push(field.name.clone());
            }

            if column_def.has_index {
                indexes.push(field.name.clone());
            }
        }

        sql.push_str(&columns.join(",\n"));

        if let Some(pk) = primary_key {
            sql.push_str(&format!(",\n    PRIMARY KEY ({})", pk));
        }

        for unique_field in unique_constraints {
            sql.push_str(&format!(",\n    UNIQUE ({})", unique_field));
        }

        if let Some(schema) = schema {
            for field in &model.fields {
                if let FieldType::Custom(ref model_name) = field.field_type {
                    if schema.models.iter().any(|m| m.name == *model_name) {
                        if let Some(rel_attr) = field.attributes.iter().find(|a| a.name == "relation") {
                            if let Some(fk_field_name) = rel_attr.args.first() {
                                if let Some(related_model) = schema.models.iter().find(|m| m.name == *model_name) {
                                    let pk_field = related_model
                                        .fields
                                        .iter()
                                        .find(|f| f.attributes.iter().any(|a| a.name == "id"))
                                        .map(|f| f.name.clone())
                                        .unwrap_or_else(|| "id".to_string());
                                    
                                    let related_table = Self::get_table_name(related_model);
                                    
                                    sql.push_str(&format!(
                                        ",\n    FOREIGN KEY ({}) REFERENCES {}({}) ON DELETE CASCADE",
                                        fk_field_name, related_table, pk_field
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        sql.push_str("\n);\n");

        for index_field in indexes {
            sql.push_str(&format!(
                "CREATE INDEX IF NOT EXISTS idx_{}_{} ON {} ({});\n",
                table_name, index_field, table_name, index_field
            ));
        }

        Ok(sql)
    }

    fn field_to_column(field: &Field, db_type: &DatabaseType) -> Result<ColumnDefinition> {
        let mut column_type = match &field.field_type {
            FieldType::Int => match db_type {
                DatabaseType::Postgres => "BIGINT".to_string(),
                DatabaseType::Sqlite => "INTEGER".to_string(),
                DatabaseType::MySql => "BIGINT".to_string(),
            },
            FieldType::String => match db_type {
                DatabaseType::Postgres => "TEXT".to_string(),
                DatabaseType::Sqlite => "TEXT".to_string(),
                DatabaseType::MySql => "TEXT".to_string(),
            },
            FieldType::Boolean => match db_type {
                DatabaseType::Postgres => "BOOLEAN".to_string(),
                DatabaseType::Sqlite => "INTEGER".to_string(),
                DatabaseType::MySql => "BOOLEAN".to_string(),
            },
            FieldType::Float => match db_type {
                DatabaseType::Postgres => "DOUBLE PRECISION".to_string(),
                DatabaseType::Sqlite => "REAL".to_string(),
                DatabaseType::MySql => "DOUBLE".to_string(),
            },
            FieldType::DateTime => match db_type {
                DatabaseType::Postgres => "TIMESTAMP".to_string(),
                DatabaseType::Sqlite => "DATETIME".to_string(),
                DatabaseType::MySql => "DATETIME".to_string(),
            },
            FieldType::Json => match db_type {
                DatabaseType::Postgres => "JSONB".to_string(),
                DatabaseType::Sqlite => "TEXT".to_string(),
                DatabaseType::MySql => "JSON".to_string(),
            },
            FieldType::Custom(_) => {
                match db_type {
                    DatabaseType::Postgres => "BIGINT".to_string(),
                    DatabaseType::Sqlite => "INTEGER".to_string(),
                    DatabaseType::MySql => "BIGINT".to_string(),
                }
            }
            FieldType::Array(_) => {
                match db_type {
                    DatabaseType::Postgres => "JSONB".to_string(),
                    DatabaseType::Sqlite => "TEXT".to_string(),
                    DatabaseType::MySql => "JSON".to_string(),
                }
            }
        };

        let is_primary = field.attributes.iter().any(|attr| attr.name == "id");
        let is_auto = field.attributes.iter().any(|attr| attr.name == "auto");
        
        if is_primary && is_auto && *db_type == DatabaseType::Sqlite {
            column_type = "INTEGER".to_string();
            column_type.push_str(" PRIMARY KEY AUTOINCREMENT");
        } else if is_auto {
            column_type = match db_type {
                DatabaseType::Postgres => format!("{} SERIAL", column_type),
                DatabaseType::Sqlite => format!("{} AUTOINCREMENT", column_type),
                DatabaseType::MySql => format!("{} AUTO_INCREMENT", column_type),
            };
        }

        if !field.optional {
            column_type.push_str(" NOT NULL");
        }

        if let Some(default_attr) = field.attributes.iter().find(|attr| attr.name == "default") {
            if let Some(default_value) = default_attr.args.first() {
                if default_value == "now" {
                    match db_type {
                        DatabaseType::Postgres => column_type.push_str(" DEFAULT CURRENT_TIMESTAMP"),
                        DatabaseType::Sqlite => column_type.push_str(" DEFAULT CURRENT_TIMESTAMP"),
                        DatabaseType::MySql => column_type.push_str(" DEFAULT CURRENT_TIMESTAMP"),
                    }
                } else {
                    column_type.push_str(&format!(" DEFAULT '{}'", default_value));
                }
            }
        }

        let is_unique = field.attributes.iter().any(|attr| attr.name == "unique");
        let has_index = field.attributes.iter().any(|attr| attr.name == "index");

        Ok(ColumnDefinition {
            column: format!("{} {}", field.name, column_type),
            is_primary,
            is_unique,
            has_index,
        })
    }

    fn generate_join_tables(
        schema: &Schema,
        _db_type: &DatabaseType,
    ) -> Result<JoinTablesResult> {
        let mut up_sql = String::new();
        let mut down_sql = String::new();
        let mut processed_join_tables = std::collections::HashSet::new();

        for model in &schema.models {
            for field in &model.fields {
                if let FieldType::Array(inner) = &field.field_type {
                    if let FieldType::Custom(related_model) = inner.as_ref() {
                        let is_one_to_many = schema
                            .models
                            .iter()
                            .find(|m| m.name == *related_model)
                            .map(|related_model| {
                                related_model.fields.iter().any(|f| {
                                    if let FieldType::Custom(ref rel_model_name) = f.field_type {
                                        if *rel_model_name == model.name {
                                            f.attributes.iter().any(|a| a.name == "relation")
                                        } else {
                                            false
                                        }
                                    } else {
                                        false
                                    }
                                })
                            })
                            .unwrap_or(false);

                        if is_one_to_many {
                            continue;
                        }

                        let (model1_name, model2_name) = if model.name < *related_model {
                            (&model.name, related_model)
                        } else {
                            (related_model, &model.name)
                        };
                        let join_table_name = Self::get_join_table_name(model1_name, model2_name);
                        
                        if processed_join_tables.contains(&join_table_name) {
                            continue;
                        }
                        processed_join_tables.insert(join_table_name.clone());
                        
                        let model1 = schema.models.iter().find(|m| m.name == *model1_name).unwrap();
                        let model2 = schema.models.iter().find(|m| m.name == *model2_name).unwrap();
                        
                        let table1 = Self::get_table_name(model1);
                        let table2 = Self::get_table_name(model2);

                        let pk1 = model1
                            .fields
                            .iter()
                            .find(|f| f.attributes.iter().any(|a| a.name == "id"))
                            .map(|f| f.name.clone())
                            .unwrap_or_else(|| "id".to_string());

                        let pk2 = model2
                            .fields
                            .iter()
                            .find(|f| f.attributes.iter().any(|a| a.name == "id"))
                            .map(|f| f.name.clone())
                            .unwrap_or_else(|| "id".to_string());

                        let fk1 = format!("{}_id", model1_name.to_lowercase());
                        let fk2 = format!("{}_id", model2_name.to_lowercase());

                        up_sql.push_str(&format!(
                            "CREATE TABLE IF NOT EXISTS {} (\n",
                            join_table_name
                        ));
                        up_sql.push_str(&format!("    {} BIGINT NOT NULL,\n", fk1));
                        up_sql.push_str(&format!("    {} BIGINT NOT NULL,\n", fk2));
                        up_sql.push_str(&format!(
                            "    PRIMARY KEY ({}, {}),\n",
                            fk1, fk2
                        ));
                        up_sql.push_str(&format!(
                            "    FOREIGN KEY ({}) REFERENCES {}({}) ON DELETE CASCADE,\n",
                            fk1, table1, pk1
                        ));
                        up_sql.push_str(&format!(
                            "    FOREIGN KEY ({}) REFERENCES {}({}) ON DELETE CASCADE\n",
                            fk2, table2, pk2
                        ));
                        up_sql.push_str(");\n\n");

                        down_sql.push_str(&format!("DROP TABLE IF EXISTS {};\n", join_table_name));
                    }
                }
            }
        }

        Ok(JoinTablesResult { up: up_sql, down: down_sql })
    }

    fn get_table_name(model: &Model) -> String {
        if let Some(table_attr) = model.attributes.iter().find(|attr| attr.name == "table") {
            if let Some(table_name) = table_attr.args.first() {
                return table_name.clone();
            }
        }
        format!("{}s", model.name.to_lowercase())
    }

    fn get_table_name_from_model_name(model_name: &str) -> String {
        format!("{}s", model_name.to_lowercase())
    }

    fn get_join_table_name(model1: &str, model2: &str) -> String {
        let mut names = vec![model1.to_lowercase(), model2.to_lowercase()];
        names.sort();
        format!("{}_{}", names[0], names[1])
    }

    pub fn create_migration(&self, name: &str) -> Result<Migration> {
        fs::create_dir_all(&self.migrations_dir)
            .map_err(|e| Error::Codegen(format!("Failed to create migrations directory: {}", e)))?;

        let timestamp = Utc::now().timestamp();
        let migration_name = format!("{}_{}", timestamp, name.replace(" ", "_").to_lowercase());
        let file_path = self.migrations_dir.join(format!("{}.sql", migration_name));

        let migration = Migration {
            name: migration_name.clone(),
            timestamp,
            up_sql: format!("-- Migration: {}\n-- Up migration\n", name),
            down_sql: format!("-- Migration: {}\n-- Down migration\n", name),
        };

        let content = format!(
            "-- Up Migration\n{}\n\n-- Down Migration\n{}",
            migration.up_sql, migration.down_sql
        );
        fs::write(&file_path, content)
            .map_err(|e| Error::Codegen(format!("Failed to write migration file: {}", e)))?;

        Ok(migration)
    }

    pub async fn apply_migration(&self, migration: &Migration) -> Result<()> {
        if self.is_migration_applied(&migration.name).await? {
            return Ok(());
        }

        let statements: Vec<&str> = migration.up_sql
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        
        for statement in statements {
            if !statement.trim().is_empty() {
                self.database.execute(statement).await?;
            }
        }

        let insert_sql = match self.database.database_type() {
            DatabaseType::Postgres | DatabaseType::MySql => format!(
                "INSERT INTO _rohas_migrations (name, timestamp) VALUES ('{}', {})",
                migration.name, migration.timestamp
            ),
            DatabaseType::Sqlite => format!(
                "INSERT INTO _rohas_migrations (name, timestamp) VALUES ('{}', {})",
                migration.name, migration.timestamp
            ),
        };

        self.database.execute(&insert_sql).await?;
        Ok(())
    }

    pub async fn rollback_migration(&self, migration: &Migration) -> Result<()> {
        if !self.is_migration_applied(&migration.name).await? {
            return Ok(());
        }

        let statements: Vec<&str> = migration.down_sql
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        
        for statement in statements {
            if !statement.trim().is_empty() {
                self.database.execute(statement).await?;
            }
        }

        let delete_sql = format!("DELETE FROM _rohas_migrations WHERE name = '{}'", migration.name);
        self.database.execute(&delete_sql).await?;

        Ok(())
    }

    async fn is_migration_applied(&self, name: &str) -> Result<bool> {
        use crate::query::Query;
        use crate::query_builder::QueryBuilder;
        let query = QueryBuilder::select_all()
            .from("_rohas_migrations")
            .where_eq("name", name)
            .limit(1);
        
        let results = query.execute(&self.database).await?;
        Ok(!results.is_empty())
    }

    pub async fn get_applied_migrations(&self) -> Result<Vec<String>> {
        use crate::query::Query;
        use crate::query_builder::QueryBuilder;
        let query = QueryBuilder::select(&["name"])
            .from("_rohas_migrations")
            .order_by("timestamp", "ASC");
        
        let results = query.execute(&self.database).await?;
        let mut migrations = Vec::new();
        
        for row in results {
            if let serde_json::Value::Object(map) = row {
                if let Some(name_value) = map.get("name").or_else(|| map.get("column_0")) {
                    if let serde_json::Value::String(name) = name_value {
                        migrations.push(name.clone());
                    }
                }
            }
        }
        
        Ok(migrations)
    }
}

struct ColumnDefinition {
    column: String,
    is_primary: bool,
    is_unique: bool,
    has_index: bool,
}

struct JoinTablesResult {
    up: String,
    down: String,
}
