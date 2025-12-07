//! Rohas ORM - A modern ORM with Rust macros, Python annotations, codegen, and query builder
//!
//! # Features
//!
//! - **Rust Macros**: Derive macros for models, queries, and relationships
//! - **Python Annotations**: Full Python support via PyO3
//! - **Code Generation**: Generate type-safe models from schemas
//! - **Query Builder**: Fluent API for building complex queries
//! - **Multi-database**: Support for PostgreSQL, MySQL, and SQLite
//! - **Async/Await**: Built on Tokio for async operations
//!
//! # Example (Rust)
//!
//! ```rust,no_run
//! use rohas_orm::prelude::*;
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Model, Debug, Clone, Serialize, Deserialize)]
//! #[table_name = "users"]
//! struct User {
//!     #[primary_key]
//!     id: i64,
//!     name: String,
//!     email: String,
//!     created_at: chrono::DateTime<chrono::Utc>,
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let db = Database::connect("postgresql://localhost/mydb").await?;
//!     
//!     let user = User::find_by_id(&db, 1).await?;
//!     println!("User: {:?}", user);
//!     
//!     Ok(())
//! }
//! ```
//!
//! # Example (Python)
//!
//! ```python
//! from rohas_orm import Model, Database, Field, Table, Index, Unique
//! 
//! @Table(name="users")
//! @Index(name="idx_email", fields=["email"])
//! @Unique(fields=["email"])
//! class User(Model):
//!     id: int = Field(primary_key=True)
//!     name: str
//!     email: str
//!     created_at: datetime
//!
//! async def main():
//!     db = await Database.connect("postgresql://localhost/mydb")
//!     user = await User.find_by_id(db, 1)
//!     print(f"User: {user}")
//! ```

pub mod connection;
pub mod error;
pub mod model;
pub mod query;
pub mod query_builder;
pub mod codegen;
pub mod python;
pub mod migration;

pub use codegen::{Codegen, Relationship, RelationshipType};

pub use python::*;

pub use connection::Database;
pub use error::{Error, Result};
pub use model::Model;
pub use query::Query;
pub use query_builder::QueryBuilder;
pub use migration::{Migration, MigrationManager};

pub mod prelude {
    pub use crate::codegen::{Codegen, Relationship, RelationshipType};
    pub use crate::connection::Database;
    pub use crate::error::{Error, Result};
    pub use crate::model::Model;
    pub use crate::query::Query;
    pub use crate::query_builder::QueryBuilder;
    pub use rohas_orm_macros::*;
}

