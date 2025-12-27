use anyhow::Result;
use rusqlite::Connection;

/// Migration 002: Add host config columns
pub fn up(conn: &Connection) -> Result<()> {
    // These will fail silently if columns already exist (which is fine)
    let _ = conn.execute("ALTER TABLE host_info ADD COLUMN ip TEXT", []);
    let _ = conn.execute("ALTER TABLE host_info ADD COLUMN hostname_field TEXT", []);
    let _ = conn.execute("ALTER TABLE host_info ADD COLUMN tailscale TEXT", []);
    let _ = conn.execute("ALTER TABLE host_info ADD COLUMN backup_path TEXT", []);
    Ok(())
}

/// Rollback: Remove host config columns
pub fn down(conn: &Connection) -> Result<()> {
    // SQLite doesn't support DROP COLUMN directly, so we need to recreate the table
    // This is a simplified rollback - in production you might want to preserve data
    conn.execute(
        "CREATE TABLE IF NOT EXISTS host_info_backup AS SELECT hostname, last_provisioned_at, docker_version, tailscale_installed, portainer_installed, metadata FROM host_info",
        [],
    )?;

    conn.execute("DROP TABLE host_info", [])?;

    conn.execute(
        "CREATE TABLE host_info (
            hostname TEXT PRIMARY KEY,
            last_provisioned_at INTEGER,
            docker_version TEXT,
            tailscale_installed INTEGER,
            portainer_installed INTEGER,
            metadata TEXT
        )",
        [],
    )?;

    conn.execute(
        "INSERT INTO host_info SELECT hostname, last_provisioned_at, docker_version, tailscale_installed, portainer_installed, metadata FROM host_info_backup",
        [],
    )?;

    conn.execute("DROP TABLE host_info_backup", [])?;

    Ok(())
}
