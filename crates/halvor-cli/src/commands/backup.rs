use anyhow::{Context, Result};
use halvor_db as db;
use std::fs;
use std::path::Path;
use chrono::Utc;

/// Handle backup command
/// hostname: None = local, Some(hostname) = remote host
/// TODO: Implement backup functionality in halvor-agent
pub fn handle_backup(
    _hostname: Option<&str>,
    _service: Option<&str>,
    _env: bool,
    _list: bool,
) -> Result<()> {
    anyhow::bail!("Backup functionality not yet implemented in halvor-agent. This will be added in a future update.");
}

/// Handle database backup
pub fn handle_backup_db(path: Option<&str>) -> Result<()> {
    let db_path = db::get_db_path()?;
    
    if !db_path.exists() {
        anyhow::bail!("Database not found at: {}", db_path.display());
    }

    let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
    let backup_path = if let Some(p) = path {
        Path::new(p).to_path_buf()
    } else {
        std::env::current_dir()?.join(format!("halvor-backup-{}.db", timestamp))
    };

    // Create parent directory if it doesn't exist
    if let Some(parent) = backup_path.parent() {
        fs::create_dir_all(parent)?;
    }

    println!("Backing up database...");
    println!("  Source: {}", db_path.display());
    println!("  Destination: {}", backup_path.display());

    fs::copy(&db_path, &backup_path)
        .with_context(|| format!("Failed to copy database to {}", backup_path.display()))?;

    println!("✓ Database backup created: {}", backup_path.display());
    Ok(())
}

/// Handle restore command
pub fn handle_restore(
    _hostname: Option<&str>,
    _service: Option<&str>,
    _env: bool,
    _backup: Option<&str>,
) -> Result<()> {
    anyhow::bail!("Restore functionality not yet implemented in halvor-agent. This will be added in a future update.");
}
