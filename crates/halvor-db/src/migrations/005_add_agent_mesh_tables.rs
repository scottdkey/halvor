use anyhow::{Context, Result};
use rusqlite::Connection;

/// Migration 005: Add agent mesh security tables
pub fn up(conn: &Connection) -> Result<()> {
    // Table for tracking trusted peer agents in the mesh
    conn.execute(
        "CREATE TABLE IF NOT EXISTS agent_peers (
            id TEXT PRIMARY KEY,
            hostname TEXT NOT NULL UNIQUE,
            tailscale_ip TEXT,
            tailscale_hostname TEXT,
            public_key TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'active',
            last_seen_at INTEGER,
            joined_at INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        [],
    )
    .context("Failed to create agent_peers table")?;

    // Table for join tokens (temporary, expire after use or timeout)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS join_tokens (
            id TEXT PRIMARY KEY,
            token TEXT NOT NULL UNIQUE,
            issuer_hostname TEXT NOT NULL,
            expires_at INTEGER NOT NULL,
            used INTEGER NOT NULL DEFAULT 0,
            used_by_hostname TEXT,
            used_at INTEGER,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        [],
    )
    .context("Failed to create join_tokens table")?;

    // Table for per-peer encryption keys (for secure agent-to-agent communication)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS peer_keys (
            id TEXT PRIMARY KEY,
            peer_hostname TEXT NOT NULL UNIQUE,
            shared_secret TEXT NOT NULL,
            algorithm TEXT NOT NULL DEFAULT 'aes-256-gcm',
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            FOREIGN KEY(peer_hostname) REFERENCES agent_peers(hostname) ON DELETE CASCADE
        )",
        [],
    )
    .context("Failed to create peer_keys table")?;

    // Index for faster lookups
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_agent_peers_hostname ON agent_peers(hostname)",
        [],
    )
    .context("Failed to create agent_peers hostname index")?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_join_tokens_token ON join_tokens(token)",
        [],
    )
    .context("Failed to create join_tokens token index")?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_join_tokens_expires_at ON join_tokens(expires_at)",
        [],
    )
    .context("Failed to create join_tokens expires_at index")?;

    Ok(())
}

/// Rollback migration 005
pub fn down(conn: &Connection) -> Result<()> {
    conn.execute("DROP INDEX IF EXISTS idx_join_tokens_expires_at", [])
        .context("Failed to drop join_tokens expires_at index")?;

    conn.execute("DROP INDEX IF EXISTS idx_join_tokens_token", [])
        .context("Failed to drop join_tokens token index")?;

    conn.execute("DROP INDEX IF EXISTS idx_agent_peers_hostname", [])
        .context("Failed to drop agent_peers hostname index")?;

    conn.execute("DROP TABLE IF EXISTS peer_keys", [])
        .context("Failed to drop peer_keys table")?;

    conn.execute("DROP TABLE IF EXISTS join_tokens", [])
        .context("Failed to drop join_tokens table")?;

    conn.execute("DROP TABLE IF EXISTS agent_peers", [])
        .context("Failed to drop agent_peers table")?;

    Ok(())
}
