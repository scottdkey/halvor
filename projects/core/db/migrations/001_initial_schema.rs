use anyhow::{Context, Result};
use rusqlite::Connection;

/// Migration 001: Initial schema
pub fn up(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            id TEXT PRIMARY KEY,
            key TEXT NOT NULL UNIQUE,
            value TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        [],
    )
    .context("Failed to create settings table")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS update_history (
            id TEXT PRIMARY KEY,
            version TEXT NOT NULL,
            channel TEXT NOT NULL,
            installed_at INTEGER NOT NULL,
            source TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        [],
    )
    .context("Failed to create update_history table")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS host_info (
            id TEXT PRIMARY KEY,
            hostname TEXT NOT NULL UNIQUE,
            last_provisioned_at INTEGER,
            docker_version TEXT,
            tailscale_installed INTEGER,
            portainer_installed INTEGER,
            metadata TEXT,
            ip TEXT,
            hostname_field TEXT,
            tailscale TEXT,
            backup_path TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        [],
    )
    .context("Failed to create host_info table")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS encrypted_env_data (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            hostname TEXT,
            key TEXT NOT NULL,
            encrypted_value TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            UNIQUE(hostname, key)
        )",
        [],
    )
    .context("Failed to create encrypted_env_data table")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at INTEGER NOT NULL
        )",
        [],
    )
    .context("Failed to create migrations table")?;

    Ok(())
}
