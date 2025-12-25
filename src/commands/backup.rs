use crate::config;
use crate::services::backup;
use anyhow::Result;
use std::io::Write;

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
        // Backup specific app/service
        backup::backup_service(target_host, service, &config)?;
    } else {
        // Backup everything on the system
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Backup All Services");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!();
        println!("This will backup all services and configuration on {}.", target_host);
        println!();
        
        // Use interactive backup which will show all available services
        // This already handles backing up all services
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
        // Restore specific app/service - require confirmation
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Restore Service: {}", service);
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!();
        println!("⚠️  WARNING: This will restore {} on {}.", service, target_host);
        println!("   Existing data may be overwritten.");
        println!();
        print!("Continue? [y/N]: ");
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let response = input.trim().to_lowercase();
        if response != "y" && response != "yes" {
            println!("Restore cancelled.");
            return Ok(());
        }
        println!();
        backup::restore_service(target_host, service, backup, &config)?;
    } else {
        // Restore everything - require confirmation
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Restore All Services");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!();
        println!("⚠️  WARNING: This will restore all services on {}.", target_host);
        println!("   Existing data may be overwritten.");
        println!();
        print!("Continue? [y/N]: ");
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let response = input.trim().to_lowercase();
        if response != "y" && response != "yes" {
            println!("Restore cancelled.");
            return Ok(());
        }
        println!();
        // Use interactive restore which will show all available backups
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
