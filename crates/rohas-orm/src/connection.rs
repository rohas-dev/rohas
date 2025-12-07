use crate::error::{Error, Result};
use sqlx::{postgres::PgPoolOptions, sqlite::{SqlitePoolOptions, SqliteConnectOptions}, mysql::MySqlPoolOptions, Pool, Postgres, Sqlite, MySql};
use std::sync::Arc;
use std::str::FromStr;
use tracing::{debug, info};

#[derive(Clone)]
pub enum DatabasePool {
    Postgres(Pool<Postgres>),
    Sqlite(Pool<Sqlite>),
    MySql(Pool<MySql>),
}

#[derive(Clone)]
pub struct Database {
    pool: Arc<DatabasePool>,
    database_type: DatabaseType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatabaseType {
    Postgres,
    Sqlite,
    MySql,
}

impl Database {
    /// Connect to a database using a connection string
    ///
    /// Supports:
    /// - `postgresql://...` or `postgres://...` for PostgreSQL
    /// - `sqlite://...` or `sqlite:...` for SQLite
    /// - `mysql://...` or `mariadb://...` for MySQL/MariaDB
    pub async fn connect(url: &str) -> Result<Self> {
        info!("Connecting to database: {}", url);
        
        let (pool, db_type) = if url.starts_with("postgresql://") || url.starts_with("postgres://") {
            let pool = PgPoolOptions::new()
                .max_connections(10)
                .connect(url)
                .await
                .map_err(|e| Error::Connection(format!("Failed to connect to PostgreSQL: {}", e)))?;
            (DatabasePool::Postgres(pool), DatabaseType::Postgres)
        } else if url.starts_with("sqlite://") || url.starts_with("sqlite:") {
            let sqlite_url = url.replace("sqlite://", "").replace("sqlite:", "");
            let connection_string = format!("sqlite://{}", sqlite_url);
            let options = SqliteConnectOptions::from_str(&connection_string)
                .map_err(|e| Error::Connection(format!("Failed to parse SQLite URL: {}", e)))?
                .create_if_missing(true);
            let pool = SqlitePoolOptions::new()
                .max_connections(10)
                .connect_with(options)
                .await
                .map_err(|e| Error::Connection(format!("Failed to connect to SQLite: {}", e)))?;
            (DatabasePool::Sqlite(pool), DatabaseType::Sqlite)
        } else if url.starts_with("mysql://") || url.starts_with("mariadb://") {
            let pool = MySqlPoolOptions::new()
                .max_connections(10)
                .connect(url)
                .await
                .map_err(|e| Error::Connection(format!("Failed to connect to MySQL: {}", e)))?;
            (DatabasePool::MySql(pool), DatabaseType::MySql)
        } else {
            return Err(Error::Connection(format!("Unsupported database URL: {}", url)));
        };

        info!("Successfully connected to database");
        
        Ok(Self {
            pool: Arc::new(pool),
            database_type: db_type,
        })
    }

    pub fn database_type(&self) -> DatabaseType {
        self.database_type
    }

    pub fn postgres_pool(&self) -> Result<&Pool<Postgres>> {
        match self.pool.as_ref() {
            DatabasePool::Postgres(pool) => Ok(pool),
            _ => Err(Error::Connection("Not a PostgreSQL database".to_string())),
        }
    }

    pub fn sqlite_pool(&self) -> Result<&Pool<Sqlite>> {
        match self.pool.as_ref() {
            DatabasePool::Sqlite(pool) => Ok(pool),
            _ => Err(Error::Connection("Not a SQLite database".to_string())),
        }
    }

    pub fn mysql_pool(&self) -> Result<&Pool<MySql>> {
        match self.pool.as_ref() {
            DatabasePool::MySql(pool) => Ok(pool),
            _ => Err(Error::Connection("Not a MySQL database".to_string())),
        }
    }

    pub async fn execute(&self, query: &str) -> Result<u64> {
        debug!("Executing query: {}", query);
        
        match &*self.pool {
            DatabasePool::Postgres(pool) => {
                let result = sqlx::query(query).execute(pool).await?;
                Ok(result.rows_affected())
            }
            DatabasePool::Sqlite(pool) => {
                let result = sqlx::query(query).execute(pool).await?;
                Ok(result.rows_affected())
            }
            DatabasePool::MySql(pool) => {
                let result = sqlx::query(query).execute(pool).await?;
                Ok(result.rows_affected())
            }
        }
    }
}

