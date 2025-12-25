use anyhow::Result;
use rusqlite::{Connection, Row};

/// Database client for executing custom SQL queries
pub struct DbClient {
    conn: Connection,
}

impl DbClient {
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }

    /// Execute a SQL statement
    pub fn execute(&self, sql: &str, params: &[&dyn rusqlite::types::ToSql]) -> Result<usize> {
        Ok(self
            .conn
            .execute(sql, rusqlite::params_from_iter(params.iter().copied()))?)
    }

    /// Query a single row with custom mapping
    pub fn query_one<T, F>(
        &self,
        sql: &str,
        params: &[&dyn rusqlite::types::ToSql],
        mut f: F,
    ) -> Result<Option<T>>
    where
        F: FnMut(&Row) -> rusqlite::Result<T>,
    {
        let mut stmt = self.conn.prepare(sql)?;
        let mut rows = stmt.query_map(params, |row| f(row))?;

        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    }

    /// Query multiple rows with custom mapping
    pub fn query_many<T, F>(
        &self,
        sql: &str,
        params: &[&dyn rusqlite::types::ToSql],
        f: F,
    ) -> Result<Vec<T>>
    where
        F: FnMut(&Row) -> rusqlite::Result<T>,
    {
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(params, f)?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Get the underlying connection (for advanced use cases)
    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}
