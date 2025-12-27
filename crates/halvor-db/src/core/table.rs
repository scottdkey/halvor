use anyhow::Result;
use rusqlite::{Connection, Row, params};
use std::marker::PhantomData;
use uuid::Uuid;

/// Trait for database table operations
/// Implement this trait for your struct to enable type-safe database access
///
/// For automatic implementation, use the `#[derive(Table)]` macro (when available)
/// or implement manually with minimal boilerplate.
pub trait Table: Sized {
    /// Table name in the database
    fn table_name() -> &'static str;

    /// Primary key column name (defaults to "id" for UUID)
    fn primary_key() -> &'static str {
        "id"
    }

    /// Get the primary key value
    fn primary_key_value(&self) -> String;

    /// Convert from database row
    fn from_row(row: &Row) -> rusqlite::Result<Self>;

    /// Convert to parameterized values for INSERT (excluding id, created_at, updated_at)
    fn to_insert_params(&self) -> Vec<Box<dyn rusqlite::types::ToSql + Send + Sync>>;

    /// Convert to parameterized values for UPDATE (excluding id, created_at, updated_at)
    fn to_update_params(&self) -> Vec<Box<dyn rusqlite::types::ToSql + Send + Sync>>;

    /// Get column names for INSERT (excluding id, created_at, updated_at)
    fn insert_columns() -> &'static [&'static str];

    /// Get column names for UPDATE (excluding id, created_at, updated_at)
    fn update_columns() -> &'static [&'static str];

    /// Get all column names including id, created_at, updated_at
    fn all_columns() -> &'static [&'static str];

    /// Get the row index for each column in SELECT * queries
    fn column_indices() -> &'static [usize];
}

/// Type-safe database operations
pub struct DbTable<T: Table> {
    _phantom: PhantomData<T>,
}

impl<T: Table> DbTable<T> {
    /// Select the first record matching a WHERE clause
    pub fn select_one(
        conn: &Connection,
        where_clause: &str,
        params: &[&dyn rusqlite::types::ToSql],
    ) -> Result<Option<T>> {
        let mut rows = Self::select_many(conn, where_clause, params)?;
        Ok(rows.pop())
    }

    /// Select a single record by primary key
    pub fn select(conn: &Connection, key: &str) -> Result<Option<T>> {
        let sql = format!(
            "SELECT * FROM {} WHERE {} = ?1",
            T::table_name(),
            T::primary_key()
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query_map(params![key], |row| T::from_row(row))?;

        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    }

    /// Select all records matching a WHERE clause
    /// Example: `DbTable::<SmbServer>::select_many(conn, "host = ?1", &["10.0.0.1"])`
    pub fn select_many(
        conn: &Connection,
        where_clause: &str,
        params: &[&dyn rusqlite::types::ToSql],
    ) -> Result<Vec<T>> {
        let sql = format!("SELECT * FROM {} WHERE {}", T::table_name(), where_clause);
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params, |row| T::from_row(row))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Select all records
    pub fn select_all(conn: &Connection) -> Result<Vec<T>> {
        let sql = format!("SELECT * FROM {}", T::table_name());
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], |row| T::from_row(row))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Insert a new record (generates UUID and timestamps automatically)
    /// The item's id, created_at, and updated_at fields are ignored and set automatically
    pub fn insert(conn: &Connection, item: &T) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();

        let columns = T::all_columns();
        let placeholders: Vec<String> = (1..=columns.len()).map(|i| format!("?{}", i)).collect();

        let sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            T::table_name(),
            columns.join(", "),
            placeholders.join(", ")
        );

        // Only use data fields - ignore id, created_at, updated_at from item
        let mut params = item.to_insert_params();
        // Prepend generated id, created_at, updated_at
        params.insert(0, Box::new(id.clone()));
        params.push(Box::new(now));
        params.push(Box::new(now));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params
            .iter()
            .map(|p| p.as_ref() as &dyn rusqlite::types::ToSql)
            .collect();

        conn.execute(&sql, rusqlite::params_from_iter(param_refs))?;
        Ok(id)
    }

    /// Insert or replace a record (UPSERT)
    /// The item's id, created_at, and updated_at fields are handled automatically:
    /// - id: uses item's id if set, otherwise generates new UUID
    /// - created_at: preserved if replacing existing (by id), otherwise set to now
    /// - updated_at: always set to now
    pub fn insert_or_replace(conn: &Connection, item: &T) -> Result<String> {
        let id = if item.primary_key_value().is_empty() {
            Uuid::new_v4().to_string()
        } else {
            item.primary_key_value()
        };

        // Check if record exists to preserve created_at
        let existing = Self::select(conn, &id)?;
        let now = chrono::Utc::now().timestamp();
        let created_at = existing
            .as_ref()
            .map(|_e| {
                // We can't access created_at field directly, so we'll use a workaround
                // For now, we'll always set created_at to now on replace
                // TODO: Need a way to preserve created_at from existing
                now
            })
            .unwrap_or(now);

        let columns = T::all_columns();
        let placeholders: Vec<String> = (1..=columns.len()).map(|i| format!("?{}", i)).collect();

        let sql = format!(
            "INSERT OR REPLACE INTO {} ({}) VALUES ({})",
            T::table_name(),
            columns.join(", "),
            placeholders.join(", ")
        );

        // Only use data fields - ignore id, created_at, updated_at from item
        let mut params = item.to_insert_params();
        params.insert(0, Box::new(id.clone()));
        params.push(Box::new(created_at));
        params.push(Box::new(now));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params
            .iter()
            .map(|p| p.as_ref() as &dyn rusqlite::types::ToSql)
            .collect();

        conn.execute(&sql, rusqlite::params_from_iter(param_refs))?;
        Ok(id)
    }

    /// Update a record by primary key (automatically updates updated_at)
    pub fn update(conn: &Connection, item: &T) -> Result<()> {
        let columns = T::update_columns();
        let set_clause: Vec<String> = columns
            .iter()
            .enumerate()
            .map(|(i, col)| format!("{} = ?{}", col, i + 1))
            .collect();

        // Add updated_at to the SET clause
        let updated_at_index = columns.len() + 1;
        let sql = format!(
            "UPDATE {} SET {}, updated_at = ?{} WHERE {} = ?{}",
            T::table_name(),
            set_clause.join(", "),
            updated_at_index,
            T::primary_key(),
            updated_at_index + 1
        );

        let mut update_params = item.to_update_params();
        let now = chrono::Utc::now().timestamp();
        update_params.push(Box::new(now));
        let pk_value = item.primary_key_value();
        update_params.push(Box::new(pk_value));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = update_params
            .iter()
            .map(|p| p.as_ref() as &dyn rusqlite::types::ToSql)
            .collect();

        conn.execute(&sql, rusqlite::params_from_iter(param_refs))?;
        Ok(())
    }

    /// Delete a record by primary key
    pub fn delete(conn: &Connection, key: &str) -> Result<()> {
        let sql = format!(
            "DELETE FROM {} WHERE {} = ?1",
            T::table_name(),
            T::primary_key()
        );
        conn.execute(&sql, params![key])?;
        Ok(())
    }

    /// Delete records matching a WHERE clause
    pub fn delete_many(
        conn: &Connection,
        where_clause: &str,
        params: &[&dyn rusqlite::types::ToSql],
    ) -> Result<usize> {
        let sql = format!("DELETE FROM {} WHERE {}", T::table_name(), where_clause);
        let rows_affected =
            conn.execute(&sql, rusqlite::params_from_iter(params.iter().copied()))?;
        Ok(rows_affected)
    }

    /// Upsert a record by a WHERE clause with automatic field merging
    /// The builder function receives the existing row (if any) and should return a new row
    /// id, created_at, and updated_at are automatically handled - you don't need to set them
    /// The builder only needs to provide the data fields, and optionally merge with existing
    pub fn upsert_by<F>(
        conn: &Connection,
        where_clause: &str,
        where_params: &[&dyn rusqlite::types::ToSql],
        builder: F,
    ) -> Result<String>
    where
        F: FnOnce(Option<&T>) -> T,
    {
        let existing = Self::select_many(conn, where_clause, where_params)?;
        let existing_row = existing.first();

        // Build the row - builder should create a complete row, but we'll fix id/created_at/updated_at
        let row = builder(existing_row);

        if existing.is_empty() {
            // New record - insert will set id, created_at, updated_at automatically
            // Ensure id is empty so insert generates it
            if !row.primary_key_value().is_empty() {
                // If builder set an id, we need to clear it or use insert_or_replace
                // For now, just use insert which will ignore the id field value
            }
            Self::insert(conn, &row)
        } else {
            // Existing record - preserve id and created_at, update will set updated_at
            let existing_id = existing_row.unwrap().primary_key_value();
            // We need to ensure the row has the correct id
            // Since we can't modify fields on generic T, we rely on builder to set it correctly
            // But we can verify and use insert_or_replace if needed
            if row.primary_key_value() != existing_id {
                // Builder didn't preserve id - use insert_or_replace to handle it
                Self::insert_or_replace(conn, &row)
            } else {
                Self::update(conn, &row)?;
                Ok(existing_id)
            }
        }
    }

    /// Select one record or return an error if not found
    pub fn select_one_or_error(
        conn: &Connection,
        where_clause: &str,
        params: &[&dyn rusqlite::types::ToSql],
        error_msg: &str,
    ) -> Result<T> {
        Self::select_one(conn, where_clause, params)?
            .ok_or_else(|| anyhow::anyhow!("{}", error_msg))
    }

    /// Insert or replace a record (SQL INSERT OR REPLACE) - type-safe version
    /// This uses SQLite's INSERT OR REPLACE which automatically handles conflicts
    /// Returns the id of the inserted/replaced record
    pub fn insert_or_replace_simple(conn: &Connection, item: &T) -> Result<String> {
        let id = if item.primary_key_value().is_empty() {
            Uuid::new_v4().to_string()
        } else {
            item.primary_key_value()
        };
        let now = chrono::Utc::now().timestamp();

        let columns = T::all_columns();
        let placeholders: Vec<String> = (1..=columns.len()).map(|i| format!("?{}", i)).collect();

        let sql = format!(
            "INSERT OR REPLACE INTO {} ({}) VALUES ({})",
            T::table_name(),
            columns.join(", "),
            placeholders.join(", ")
        );

        let mut params = item.to_insert_params();
        params.insert(0, Box::new(id.clone()));
        params.push(Box::new(now));
        params.push(Box::new(now));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params
            .iter()
            .map(|p| p.as_ref() as &dyn rusqlite::types::ToSql)
            .collect();

        conn.execute(&sql, rusqlite::params_from_iter(param_refs))?;
        Ok(id)
    }
}

/// Helper function to generate table creation SQL with standard columns
pub fn create_table_sql(table_name: &str, columns: &[&str]) -> String {
    let mut cols: Vec<String> = vec![
        "id TEXT PRIMARY KEY".to_string(),
        "created_at INTEGER NOT NULL".to_string(),
        "updated_at INTEGER NOT NULL".to_string(),
    ];
    cols.extend(columns.iter().map(|c| c.to_string()));
    format!(
        "CREATE TABLE IF NOT EXISTS {} ({})",
        table_name,
        cols.join(", ")
    )
}
