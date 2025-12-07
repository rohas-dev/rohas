use rohas_orm::{Database, MigrationManager, Query, QueryBuilder};
use rohas_parser::ast::{Attribute, Field, FieldType, Model, Schema};
use tempfile::TempDir;

fn create_test_schema() -> Schema {
    let mut schema = Schema::new();
    
    let user_model = Model {
        name: "User".to_string(),
        fields: vec![
            Field {
                name: "id".to_string(),
                field_type: FieldType::Int,
                optional: false,
                attributes: vec![
                    Attribute { name: "id".to_string(), args: vec![] },
                    Attribute { name: "auto".to_string(), args: vec![] },
                ],
            },
            Field {
                name: "name".to_string(),
                field_type: FieldType::String,
                optional: false,
                attributes: vec![],
            },
            Field {
                name: "email".to_string(),
                field_type: FieldType::String,
                optional: false,
                attributes: vec![Attribute { name: "unique".to_string(), args: vec![] }],
            },
            Field {
                name: "createdAt".to_string(),
                field_type: FieldType::DateTime,
                optional: false,
                attributes: vec![Attribute { name: "default".to_string(), args: vec!["now".to_string()] }],
            },
        ],
        attributes: vec![],
    };

    // Post model with relationship
    let post_model = Model {
        name: "Post".to_string(),
        fields: vec![
            Field {
                name: "id".to_string(),
                field_type: FieldType::Int,
                optional: false,
                attributes: vec![
                    Attribute { name: "id".to_string(), args: vec![] },
                    Attribute { name: "auto".to_string(), args: vec![] },
                ],
            },
            Field {
                name: "title".to_string(),
                field_type: FieldType::String,
                optional: false,
                attributes: vec![],
            },
            Field {
                name: "content".to_string(),
                field_type: FieldType::String,
                optional: true,
                attributes: vec![],
            },
            Field {
                name: "authorId".to_string(),
                field_type: FieldType::Int,
                optional: false,
                attributes: vec![],
            },
            Field {
                name: "author".to_string(),
                field_type: FieldType::Custom("User".to_string()),
                optional: true,
                attributes: vec![
                    Attribute { name: "relation".to_string(), args: vec!["authorId".to_string()] },
                ],
            },
        ],
        attributes: vec![],
    };

    schema.models.push(user_model);
    schema.models.push(post_model);
    schema
}

async fn setup_test_db() -> (Database, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db_path_str = db_path.to_string_lossy();
    let db_url = if db_path_str.starts_with('/') {
        format!("sqlite://{}", db_path_str)
    } else {
        format!("sqlite:///{}", db_path_str)
    };
    let db = Database::connect(&db_url).await.unwrap();
    (db, temp_dir)
}

#[tokio::test]
async fn test_init_migration_table() {
    let (db, _temp_dir) = setup_test_db().await;
    let migrations_dir = std::path::PathBuf::from("migrations");
    let manager = MigrationManager::new(migrations_dir, db.clone());

    manager.init().await.unwrap();

    let query = QueryBuilder::select_all()
        .from("_rohas_migrations")
        .limit(1);
    
    let _results = query.execute(&db).await.unwrap();
    assert!(true, "Migration table created successfully");
}

#[tokio::test]
async fn test_generate_migration_from_schema() {
    let schema = create_test_schema();
    let (db, _temp_dir) = setup_test_db().await;

    let manager = MigrationManager::new(std::path::PathBuf::from("migrations"), db.clone());
    let (up_sql, down_sql) = manager.generate_migration_from_schema(
        &schema,
    ).await.unwrap();

    assert!(up_sql.contains("CREATE TABLE"), "Up migration should contain CREATE TABLE");
    assert!(up_sql.contains("users"), "Should create users table");
    assert!(up_sql.contains("posts"), "Should create posts table");
    assert!(up_sql.contains("PRIMARY KEY"), "Should have primary key");
    assert!(up_sql.contains("UNIQUE"), "Should have unique constraint");

    assert!(down_sql.contains("DROP TABLE"), "Down migration should contain DROP TABLE");
}

#[tokio::test]
async fn test_apply_migration() {
    let (db, _temp_dir) = setup_test_db().await;
    let temp_migrations = TempDir::new().unwrap();
    let migrations_dir = temp_migrations.path().to_path_buf();
    
    let manager = MigrationManager::new(migrations_dir.clone(), db.clone());
    manager.init().await.unwrap();

    let schema = create_test_schema();
    let manager = MigrationManager::new(std::path::PathBuf::from("migrations"), db.clone());
    let (up_sql, down_sql) = manager.generate_migration_from_schema(
        &schema,
    ).await.unwrap();

    let migration = rohas_orm::Migration {
        name: "test_migration".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        up_sql,
        down_sql,
    };

    manager.apply_migration(&migration).await.unwrap();

    let query = QueryBuilder::select_all()
        .from("users")
        .limit(1);
    let _results = query.execute(&db).await.unwrap();

    let query = QueryBuilder::select_all()
        .from("posts")
        .limit(1);
    let _results = query.execute(&db).await.unwrap();

    let applied = manager.get_applied_migrations().await.unwrap();
    assert!(applied.contains(&"test_migration".to_string()), "Migration should be recorded");
}

#[tokio::test]
async fn test_migration_idempotency() {
    let (db, _temp_dir) = setup_test_db().await;
    let temp_migrations = TempDir::new().unwrap();
    let migrations_dir = temp_migrations.path().to_path_buf();
    
    let manager = MigrationManager::new(migrations_dir, db.clone());
    manager.init().await.unwrap();

    let schema = create_test_schema();
    let manager = MigrationManager::new(std::path::PathBuf::from("migrations"), db.clone());
    let (up_sql, down_sql) = manager.generate_migration_from_schema(
        &schema,
    ).await.unwrap();

    let migration = rohas_orm::Migration {
        name: "test_migration_2".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        up_sql: up_sql.clone(),
        down_sql,
    };

    manager.apply_migration(&migration).await.unwrap();
    manager.apply_migration(&migration).await.unwrap();

    let applied = manager.get_applied_migrations().await.unwrap();
    let count = applied.iter().filter(|&n| n == "test_migration_2").count();
    assert_eq!(count, 1, "Migration should only be recorded once");
}

#[tokio::test]
async fn test_foreign_key_constraints() {
    let (db, _temp_dir) = setup_test_db().await;
    let temp_migrations = TempDir::new().unwrap();
    let migrations_dir = temp_migrations.path().to_path_buf();
    
    let manager = MigrationManager::new(migrations_dir, db.clone());
    manager.init().await.unwrap();

    let schema = create_test_schema();
    let (up_sql, _down_sql) = manager.generate_migration_from_schema(
        &schema,
    ).await.unwrap();

    assert!(up_sql.contains("FOREIGN KEY"), "Should contain foreign key constraint");
    assert!(up_sql.contains("authorId"), "Should reference authorId field");
    assert!(up_sql.contains("users"), "Should reference users table");

    let migration = rohas_orm::Migration {
        name: "test_fk_migration".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        up_sql,
        down_sql: "DROP TABLE IF EXISTS posts; DROP TABLE IF EXISTS users;".to_string(),
    };

    manager.apply_migration(&migration).await.unwrap();

    let insert_user_sql = "INSERT INTO users (id, name, email, createdAt) VALUES (1, 'Test User', 'test@example.com', CURRENT_TIMESTAMP)";
    db.execute(insert_user_sql).await.unwrap();

    let insert_post_sql = "INSERT INTO posts (id, title, content, authorId) VALUES (1, 'Test Post', 'Content', 1)";
    db.execute(insert_post_sql).await.unwrap();

    let query = QueryBuilder::select_all()
        .from("posts")
        .where_eq_num("id", 1);
    let results = query.execute(&db).await.unwrap();
    assert!(!results.is_empty(), "Post should be inserted");
}
