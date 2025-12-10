use anyhow::{Context, Result};
use rusqlite::Error as SqliteError;

/// Convert SQLite errors to user-friendly messages
pub fn handle_db_error(error: rusqlite::Error) -> anyhow::Error {
    match error {
        SqliteError::SqliteFailure(_err, Some(msg)) => {
            let msg_str = msg.to_string();
            if msg_str.contains("UNIQUE constraint") {
                anyhow::anyhow!("Unique constraint violation: {}", msg_str)
                    .context("A record with this value already exists")
            } else if msg_str.contains("FOREIGN KEY constraint") {
                anyhow::anyhow!("Foreign key constraint violation: {}", msg_str)
                    .context("Referenced record does not exist")
            } else if msg_str.contains("NOT NULL constraint") {
                anyhow::anyhow!("Not null constraint violation: {}", msg_str)
                    .context("A required field is missing")
            } else {
                anyhow::anyhow!("Database error: {}", msg_str)
            }
        }
        SqliteError::SqliteFailure(err, None) => {
            anyhow::anyhow!("Database error: {}", err)
        }
        _ => anyhow::anyhow!("Database error: {}", error),
    }
}

/// Execute a database operation with automatic error handling
pub fn execute_with_error_handling<F, R>(operation: F) -> Result<R>
where
    F: FnOnce() -> std::result::Result<R, rusqlite::Error>,
{
    operation()
        .map_err(handle_db_error)
        .context("Database operation failed")
}
