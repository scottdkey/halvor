use crate::config;
use crate::services::backup;
use anyhow::Result;

/// Handle backup command
/// hostname: None = local, Some(hostname) = remote host
pub fn handle_backup(
    hostname: Option<&str>,
    service: Option<&str>,
    env: bool,
    list: bool,
) -> Result<()> {
    let config = config::load_config()?;
    let target_host = hostname.unwrap_or("localhost");

    if list {
        backup::list_backups(target_host, &config)?;
    } else if env {
        backup::backup_to_env(target_host, service, &config)?;
    } else if let Some(service) = service {
        backup::backup_service(target_host, service, &config)?;
    } else {
        // Interactive backup selection
        backup::backup_interactive(target_host, &config)?;
    }
    Ok(())
}

/// Handle restore command
/// hostname: None = local, Some(hostname) = remote host
pub fn handle_restore(
    hostname: Option<&str>,
    service: Option<&str>,
    env: bool,
    backup: Option<&str>,
) -> Result<()> {
    let config = config::load_config()?;
    let target_host = hostname.unwrap_or("localhost");

    if env {
        backup::restore_from_env(target_host, service, &config)?;
    } else if let Some(service) = service {
        backup::restore_service(target_host, service, backup, &config)?;
    } else {
        // Interactive restore selection
        backup::restore_interactive(target_host, &config)?;
    }
    Ok(())
}

/// Handle database backup command
/// Requires administrator password for security
pub fn handle_backup_db(path: Option<&str>) -> Result<()> {
    use crate::config::service;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Database Backup");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("⚠️  This operation requires administrator privileges.");
    println!("   The database backup will be unencrypted (plain SQLite format).");
    println!();

    // Perform the backup (will use sudo if needed)
    service::backup_database(path)?;

    println!();
    println!("✓ Database backup complete");

    Ok(())
}
