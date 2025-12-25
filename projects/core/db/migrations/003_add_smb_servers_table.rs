use anyhow::{Context, Result};
use rusqlite::Connection;

/// Migration 003: Add SMB servers table
pub fn up(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS smb_servers (
            id TEXT PRIMARY KEY,
            server_name TEXT NOT NULL UNIQUE,
            host TEXT NOT NULL,
            shares TEXT NOT NULL,
            username TEXT,
            password TEXT,
            options TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        [],
    )
    .context("Failed to create smb_servers table")?;
    Ok(())
}

/// Rollback: Remove SMB servers table
pub fn down(conn: &Connection) -> Result<()> {
    conn.execute("DROP TABLE IF EXISTS smb_servers", [])
        .context("Failed to drop smb_servers table")?;
    Ok(())
}
