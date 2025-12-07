# rohas-orm (WIP)

A modern ORM for Rohas with Rust macros, Python annotations, code generation, and a fluent query builder.

## Features

- **Rust Macros**: Derive macros for models, queries, and relationships
- **Python Annotations**: Full Python support via PyO3
- **Code Generation**: Generate type-safe models from Rohas schemas
- **Query Builder**: Fluent API for building complex SQL queries
- **Multi-database**: Support for PostgreSQL, MySQL, and SQLite
- **Async/Await**: Built on Tokio for async operations

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
rohas-orm = { path = "../rohas-orm" }
```

## Usage

### Rust

```rust
use rohas_orm::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Model, Debug, Clone, Serialize, Deserialize)]
#[table_name = "users"]
struct User {
    #[primary_key]
    id: i64,
    name: String,
    email: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let db = Database::connect("postgresql://localhost/mydb").await?;
    
    // Find by ID
    let user = User::find_by_id(&db, 1).await?;
    println!("User: {:?}", user);
    
    // Find all
    let users = User::find_all(&db).await?;
    
    // Create new user
    let new_user = User {
        id: 0,
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
        created_at: chrono::Utc::now(),
    };
    new_user.save(&db).await?;
    
    // Query builder
    let query = QueryBuilder::select(&["id", "name"])
        .from("users")
        .where_eq("email", "john@example.com")
        .order_by("name", "ASC")
        .limit(10);
    
    let results = query.execute(&db).await?;
    
    Ok(())
}
```

### Python

```python
from rohas_orm import Model, Database, Field
from datetime import datetime

class User(Model):
    id: int = Field(primary_key=True)
    name: str
    email: str
    created_at: datetime

async def main():
    db = Database("postgresql://localhost/mydb")
    
    # Find by ID
    user = await User.find_by_id(db, 1)
    print(f"User: {user}")
    
    # Query builder
    query = QueryBuilder.select_all() \
        .from_("users") \
        .where_eq("email", "john@example.com") \
        .order_by("name", "ASC") \
        .limit(10)
    
    results = await db.query(query)
    print(results)
```

## Code Generation

Generate models from Rohas schema files with full support for relationships and attributes:

### From Schema Directory

```rust
use rohas_orm::prelude::*;

// Load all .ro files from a directory
let mut codegen = Codegen::new("src/generated".into());
codegen.load_schema_dir("schema/models")?;

// Generate Rust models
codegen.generate_rust_models()?;

// Generate Python models
codegen.generate_python_models()?;
```

### From Single Schema File

```rust
use rohas_orm::prelude::*;

let mut codegen = Codegen::new("src/generated".into());
codegen.load_schema_file("schema/models/user.ro")?;
codegen.generate_rust_models()?;
```

### Schema Features Supported

- **Attributes**: `@id`, `@auto`, `@unique`, `@default(now)`, `@relation`
- **Relationships**: 
  - One-to-One: `user User?` or `user User`
  - One-to-Many: `posts Post[]`
  - Many-to-Many: `tags Tag[]` (with join table)
  - BelongsTo: `userId Int @relation(User)`
- **Field Types**: `Int`, `String`, `Boolean`, `Float`, `DateTime`, `Json`, `Custom`
- **Optional Fields**: `email String?`

### Example Schema

```rohas
model User {
  id        Int      @id @auto
  name      String
  email     String   @unique
  createdAt DateTime @default(now)
  posts     Post[]
}

model Post {
  id        Int      @id @auto
  title     String
  content   String
  userId    Int      @relation(User)
  author    User?    @relation(userId)
  createdAt DateTime @default(now)
}
```

This will generate:
- Model structs with proper types
- Relationship loading methods (`load_posts()`, `load_author()`)
- Foreign key handling
- Attribute support (primary keys, unique constraints, defaults)

## Query Builder

The query builder provides a fluent API for constructing SQL queries:

```rust
// SELECT
let query = QueryBuilder::select(&["id", "name", "email"])
    .from("users")
    .where_eq("active", "true")
    .order_by("created_at", "DESC")
    .limit(10)
    .offset(0);

// INSERT
let query = QueryBuilder::insert("users")
    .values(vec!["1", "John Doe", "john@example.com"]);

// UPDATE
let query = QueryBuilder::update("users")
    .set("name", "Jane Doe")
    .where_eq_num("id", 1);

// DELETE
let query = QueryBuilder::delete("users")
    .where_eq_num("id", 1);
```

## Database Support

- **PostgreSQL**: `postgresql://user:pass@host/dbname`
- **MySQL**: `mysql://user:pass@host/dbname`
- **SQLite**: `sqlite://path/to/database.db`

## License

MIT OR Apache-2.0

