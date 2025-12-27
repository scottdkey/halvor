use anyhow::{Context, Result};
use rusqlite::Connection;

/// Migration 004: Fix host_info table to ensure id column exists
/// This migration handles the case where host_info was created without an id column
pub fn up(conn: &Connection) -> Result<()> {
    // Check if table exists at all
    let table_exists = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='host_info'",
            [],
            |row| {
                let count: i32 = row.get(0)?;
                Ok(count > 0)
            },
        )
        .unwrap_or(false);

    if !table_exists {
        // Table doesn't exist - create it with correct schema
        println!("host_info table does not exist, creating with correct schema...");
        conn.execute(
            "CREATE TABLE host_info (
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
        println!("✓ host_info table created");
        return Ok(());
    }

    // Check if id column exists
    let has_id_column = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('host_info') WHERE name='id'",
            [],
            |row| {
                let count: i32 = row.get(0)?;
                Ok(count > 0)
            },
        )
        .unwrap_or(false);

    if !has_id_column {
        // Table exists but doesn't have id column - need to recreate it
        println!("host_info table missing id column, recreating table...");

        // Get list of columns that actually exist in the old table
        let mut existing_columns = Vec::new();
        let mut stmt = conn
            .prepare("SELECT name FROM pragma_table_info('host_info')")
            .context("Failed to prepare pragma_table_info query")?;
        let rows = stmt
            .query_map([], |row| {
                let name: String = row.get(0)?;
                Ok(name)
            })
            .context("Failed to query table info")?;
        for row in rows {
            existing_columns.push(row.context("Failed to read column name")?);
        }

        // Backup existing data - only select columns that exist
        let column_list = existing_columns.join(", ");
        conn.execute(
            &format!(
                "CREATE TABLE IF NOT EXISTS host_info_backup AS SELECT {} FROM host_info",
                column_list
            ),
            [],
        )
        .context("Failed to create backup table")?;

        // Drop old table
        conn.execute("DROP TABLE host_info", [])
            .context("Failed to drop old host_info table")?;

        // Create new table with correct schema (including id)
        conn.execute(
            "CREATE TABLE host_info (
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
        .context("Failed to create new host_info table")?;

        // Restore data - build SELECT list with only columns that exist
        let mut select_parts = vec!["lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)), 2) || '-' || hex(randomblob(6)))".to_string()];

        let column_mapping = vec![
            ("hostname", "hostname"),
            ("last_provisioned_at", "last_provisioned_at"),
            ("docker_version", "docker_version"),
            ("tailscale_installed", "tailscale_installed"),
            ("portainer_installed", "portainer_installed"),
            ("metadata", "metadata"),
            ("ip", "ip"),
            ("hostname_field", "hostname_field"),
            ("tailscale", "tailscale"),
            ("backup_path", "backup_path"),
            ("created_at", "created_at"),
            ("updated_at", "updated_at"),
        ];

        for (_target_col, source_col) in column_mapping {
            if existing_columns.contains(&source_col.to_string()) {
                select_parts.push(source_col.to_string());
            } else {
                // Column doesn't exist in old table, use default
                if source_col == "created_at" || source_col == "updated_at" {
                    select_parts.push("strftime('%s', 'now')".to_string());
                } else {
                    select_parts.push("NULL".to_string());
                }
            }
        }

        let select_list = select_parts.join(", ");
        conn.execute(
            &format!(
                "INSERT INTO host_info (
                    id, hostname, last_provisioned_at, docker_version, 
                    tailscale_installed, portainer_installed, metadata,
                    ip, hostname_field, tailscale, backup_path,
                    created_at, updated_at
                )
                SELECT {}
                FROM host_info_backup",
                select_list
            ),
            [],
        )
        .context("Failed to restore data to new host_info table")?;

        // Drop backup table
        conn.execute("DROP TABLE host_info_backup", [])
            .context("Failed to drop backup table")?;

        println!("✓ host_info table recreated with id column");
    }

    Ok(())
}

/// Rollback: This migration is not easily reversible
pub fn down(_conn: &Connection) -> Result<()> {
    // Rollback would require removing the id column, which SQLite doesn't support easily
    // This migration is considered one-way
    anyhow::bail!("Migration 004 does not support rollback");
}
