# Database Abstraction Layer

This module provides a type-safe database abstraction layer for SQLite (and potentially PostgreSQL
in the future).

## Features

- **Type-safe CRUD operations**: Select, insert, update, delete with compile-time type checking
- **Automatic UUID primary keys**: All tables automatically get `id` (UUID), `created_at`, and
  `updated_at` columns
- **Automatic timestamp management**: `updated_at` is automatically updated on every update
  operation
- **Custom SQL support**: `DbClient` allows executing custom SQL queries with typed responses
- **Minimal boilerplate**: Implement the `Table` trait to enable all CRUD operations

## Basic Usage

### 1. Define your struct

```rust
#[derive(Debug, Clone)]
pub struct MyTable {
    pub id: String,              // UUID primary key (required)
    pub name: String,            // Your custom fields
    pub email: Option<String>,   // Optional fields supported
    pub created_at: i64,         // Auto-managed
    pub updated_at: i64,         // Auto-managed
}
```

### 2. Implement the Table trait

```rust
use crate::db::core::table::{DbTable, Table};
use rusqlite::Row;

impl Table for MyTable {
    fn table_name() -> &'static str {
        "my_table"
    }

    fn primary_key() -> &'static str {
        "id"  // Default, can be overridden
    }

    fn primary_key_value(&self) -> String {
        self.id.clone()
    }

    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(MyTable {
            id: row.get(0)?,
            name: row.get(1)?,
            email: row.get(2)?,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
        })
    }

    fn to_insert_params(&self) -> Vec<Box<dyn rusqlite::types::ToSql + Send + Sync>> {
        vec![
            Box::new(self.name.clone()),
            Box::new(self.email.clone()),
        ]
    }

    fn to_update_params(&self) -> Vec<Box<dyn rusqlite::types::ToSql + Send + Sync>> {
        vec![
            Box::new(self.name.clone()),
            Box::new(self.email.clone()),
        ]
    }

    fn insert_columns() -> &'static [&'static str] {
        &["name", "email"]
    }

    fn update_columns() -> &'static [&'static str] {
        &["name", "email"]
    }

    fn all_columns() -> &'static [&'static str] {
        &["id", "name", "email", "created_at", "updated_at"]
    }

    fn column_indices() -> &'static [usize] {
        &[0, 1, 2, 3, 4]
    }
}
```

### 3. Use the database operations

```rust
use crate::db;
use crate::db::core::table::DbTable;

// Insert
let item = MyTable {
    id: String::new(),  // Will be auto-generated
    name: "John".to_string(),
    email: Some("john@example.com".to_string()),
    created_at: 0,
    updated_at: 0,
};
let id = DbTable::<MyTable>::insert(&conn, &item)?;

// Select by primary key
let item = DbTable::<MyTable>::select(&conn, &id)?;

// Select all
let all_items = DbTable::<MyTable>::select_all(&conn)?;

// Select with WHERE clause
let items = DbTable::<MyTable>::select_many(
    &conn,
    "name = ?1",
    &[&"John" as &dyn rusqlite::types::ToSql],
)?;

// Update
item.name = "Jane".to_string();
DbTable::<MyTable>::update(&conn, &item)?;  // updated_at automatically set

// Delete
DbTable::<MyTable>::delete(&conn, &id)?;
```

## Custom SQL Queries

For complex queries that don't fit the standard CRUD pattern, use `DbClient`:

```rust
use crate::db;

let client = db::get_client()?;

// Query with custom SQL and typed response
let result: Option<String> = client.query_one(
    "SELECT name FROM my_table WHERE email = ?1",
    &[&"john@example.com" as &dyn rusqlite::types::ToSql],
    |row| Ok(row.get::<_, String>(0)?),
)?;

// Query multiple rows
let results: Vec<(String, String)> = client.query_many(
    "SELECT name, email FROM my_table WHERE created_at > ?1",
    &[&timestamp as &dyn rusqlite::types::ToSql],
    |row| Ok((row.get(0)?, row.get(1)?)),
)?;

// Execute custom SQL
client.execute(
    "UPDATE my_table SET status = ?1 WHERE id = ?2",
    &[&"active" as &dyn rusqlite::types::ToSql, &id as &dyn rusqlite::types::ToSql],
)?;
```

## Migration Example

When creating a new table via migration, use the standard columns:

```rust
pub fn up(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS my_table (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        [],
    )?;
    Ok(())
}
```

Or use the helper function:

```rust
use crate::db::core::table::create_table_sql;

let sql = create_table_sql("my_table", &["name TEXT NOT NULL", "email TEXT"]);
conn.execute(&sql, [])?;
```
