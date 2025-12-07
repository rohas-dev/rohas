use crate::connection::Database;
use crate::error::Result;
use crate::query_builder::QueryBuilder;
use sqlx::Row;

/// Query trait for building and executing queries
pub trait Query {
    async fn execute(&self, db: &Database) -> Result<Vec<serde_json::Value>>;

    async fn execute_one(&self, db: &Database) -> Result<Option<serde_json::Value>>;

    async fn execute_affected(&self, db: &Database) -> Result<u64>;

    fn to_sql(&self) -> String;
}

impl Query for QueryBuilder {
    async fn execute(&self, db: &Database) -> Result<Vec<serde_json::Value>> {
        let sql = self.to_sql();
        
        let results = match db.database_type() {
            crate::connection::DatabaseType::Postgres => {
                let pool = db.postgres_pool()?;
                let rows = sqlx::query(&sql).fetch_all(pool).await?;
                convert_pg_rows(rows)?
            }
            crate::connection::DatabaseType::Sqlite => {
                let pool = db.sqlite_pool()?;
                let rows = sqlx::query(&sql).fetch_all(pool).await?;
                convert_sqlite_rows(rows)?
            }
            crate::connection::DatabaseType::MySql => {
                let pool = db.mysql_pool()?;
                let rows = sqlx::query(&sql).fetch_all(pool).await?;
                convert_mysql_rows(rows)?
            }
        };
        
        Ok(results)
    }

    async fn execute_one(&self, db: &Database) -> Result<Option<serde_json::Value>> {
        let results = self.execute(db).await?;
        Ok(results.into_iter().next())
    }

    async fn execute_affected(&self, db: &Database) -> Result<u64> {
        let sql = self.to_sql();
        db.execute(&sql).await
    }

    fn to_sql(&self) -> String {
        QueryBuilder::to_sql(self)
    }
}

fn convert_pg_rows(rows: Vec<sqlx::postgres::PgRow>) -> Result<Vec<serde_json::Value>> {
    let mut results = Vec::new();
    for row in rows {
        let mut map = serde_json::Map::new();
        for i in 0..row.len() {
            let name = format!("column_{}", i);
            let value = get_pg_value(&row, i)?;
            map.insert(name, value);
        }
        results.push(serde_json::Value::Object(map));
    }
    Ok(results)
}

fn get_pg_value(row: &sqlx::postgres::PgRow, i: usize) -> Result<serde_json::Value> {
    if let Ok(v) = row.try_get::<i64, _>(i) {
        return Ok(serde_json::Value::Number(v.into()));
    }
    if let Ok(v) = row.try_get::<String, _>(i) {
        return Ok(serde_json::Value::String(v));
    }
    if let Ok(v) = row.try_get::<f64, _>(i) {
        return Ok(serde_json::Value::Number(
            serde_json::Number::from_f64(v).unwrap_or(serde_json::Number::from(0))
        ));
    }
    if let Ok(v) = row.try_get::<bool, _>(i) {
        return Ok(serde_json::Value::Bool(v));
    }
    Ok(serde_json::Value::Null)
}

fn convert_sqlite_rows(rows: Vec<sqlx::sqlite::SqliteRow>) -> Result<Vec<serde_json::Value>> {
    let mut results = Vec::new();
    for row in rows {
        let mut map = serde_json::Map::new();
        for i in 0..row.len() {
            let name = format!("column_{}", i);
            let value = get_sqlite_value(&row, i)?;
            map.insert(name, value);
        }
        results.push(serde_json::Value::Object(map));
    }
    Ok(results)
}

fn get_sqlite_value(row: &sqlx::sqlite::SqliteRow, i: usize) -> Result<serde_json::Value> {
    if let Ok(v) = row.try_get::<i64, _>(i) {
        return Ok(serde_json::Value::Number(v.into()));
    }
    if let Ok(v) = row.try_get::<String, _>(i) {
        return Ok(serde_json::Value::String(v));
    }
    if let Ok(v) = row.try_get::<f64, _>(i) {
        return Ok(serde_json::Value::Number(
            serde_json::Number::from_f64(v).unwrap_or(serde_json::Number::from(0))
        ));
    }
    if let Ok(v) = row.try_get::<bool, _>(i) {
        return Ok(serde_json::Value::Bool(v));
    }
    Ok(serde_json::Value::Null)
}

fn convert_mysql_rows(rows: Vec<sqlx::mysql::MySqlRow>) -> Result<Vec<serde_json::Value>> {
    let mut results = Vec::new();
    for row in rows {
        let mut map = serde_json::Map::new();
        for i in 0..row.len() {
            let name = format!("column_{}", i);
            let value = get_mysql_value(&row, i)?;
            map.insert(name, value);
        }
        results.push(serde_json::Value::Object(map));
    }
    Ok(results)
}

fn get_mysql_value(row: &sqlx::mysql::MySqlRow, i: usize) -> Result<serde_json::Value> {
    if let Ok(v) = row.try_get::<i64, _>(i) {
        return Ok(serde_json::Value::Number(v.into()));
    }
    if let Ok(v) = row.try_get::<String, _>(i) {
        return Ok(serde_json::Value::String(v));
    }
    if let Ok(v) = row.try_get::<f64, _>(i) {
        return Ok(serde_json::Value::Number(
            serde_json::Number::from_f64(v).unwrap_or(serde_json::Number::from(0))
        ));
    }
    if let Ok(v) = row.try_get::<bool, _>(i) {
        return Ok(serde_json::Value::Bool(v));
    }
    Ok(serde_json::Value::Null)
}
