use crate::config;
use crate::config::config_manager;
use crate::db;
use crate::services;
use anyhow::Result;
use std::env;
use std::io::{self, Write};
use std::path::Path;

/// Handle uninstall command for a service on a host
/// hostname: None = local, Some(hostname) = remote host
pub fn handle_uninstall(hostname: Option<&str>, service: &str) -> Result<()> {
    let config = config::load_config()?;
    let target_host = hostname.unwrap_or("localhost");

    match service.to_lowercase().as_str() {
        "npm" => {
            // TODO: Implement NPM uninstall
            anyhow::bail!("NPM uninstall not yet implemented");
        }
        "portainer" => {
            // TODO: Implement Portainer uninstall
            anyhow::bail!("Portainer uninstall not yet implemented");
        }
        "smb" => {
            services::smb::uninstall_smb_mounts(target_host, &config)?;
        }
        _ => {
            anyhow::bail!(
                "Unknown service: {}. Supported services: npm, portainer, smb",
                service
            );
        }
    }

    Ok(())
}

/// Handle guided uninstall for halvor (local or remote)
pub fn handle_guided_uninstall(hostname: Option<&str>) -> Result<()> {
    let config = config::load_config()?;
    let target_host = hostname.unwrap_or("localhost");

    // For remote hosts, we can only uninstall services, not the halvor binary itself
    if hostname.is_some() && target_host != "localhost" {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Guided Uninstall for Remote Host: {}", target_host);
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!();
        println!("For remote hosts, you can uninstall services but not the halvor binary.");
        println!("Available services to uninstall:");
        println!("  - npm (Nginx Proxy Manager)");
        println!("  - portainer (Portainer)");
        println!("  - smb (SMB mounts)");
        println!();
        print!("Enter service to uninstall (or press Enter to cancel): ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let service = input.trim();

        if service.is_empty() {
            println!("Cancelled.");
            return Ok(());
        }

        return handle_uninstall(hostname, service);
    }

    // Local uninstall - guided flow
    handle_local_guided_uninstall()
}

/// Handle guided uninstall for local machine
fn handle_local_guided_uninstall() -> Result<()> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Guided Uninstall - halvor");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("This will guide you through uninstalling halvor from your system.");
    println!("You will be prompted for each step.");
    println!();

    // Collect all binaries and backup files to remove
    let home = env::var("HOME").or_else(|_| env::var("USERPROFILE"))?;
    let locations = vec![
        format!("{}/.cargo/bin/hal", home),
        format!("{}/.cargo/bin/halvor", home),
        format!("{}/.local/bin/hal", home),
        format!("{}/.local/bin/halvor", home),
        "/usr/local/bin/hal".to_string(),
        "/usr/local/bin/halvor".to_string(),
        "/usr/bin/hal".to_string(),
        "/usr/bin/halvor".to_string(),
    ];

    let mut binaries_to_remove = Vec::new();
    let mut backups_to_remove = Vec::new();

    // Check for binaries
    for path_str in &locations {
        let path = Path::new(path_str);
        if path.exists() {
            binaries_to_remove.push(path_str.clone());
        }
    }

    // Check for backup files
    let backup_patterns = vec![
        format!("{}/.cargo/bin/hal*.bak", home),
        format!("{}/.cargo/bin/halvor*.bak", home),
        format!("{}/.local/bin/hal*.bak", home),
        format!("{}/.local/bin/halvor*.bak", home),
        "/usr/local/bin/hal*.bak".to_string(),
        "/usr/local/bin/halvor*.bak".to_string(),
        "/usr/bin/hal*.bak".to_string(),
        "/usr/bin/halvor*.bak".to_string(),
    ];

    // Use glob to find backup files
    for pattern in &backup_patterns {
        if let Ok(entries) = glob::glob(pattern) {
            for entry in entries.flatten() {
                if entry.is_file() {
                    backups_to_remove.push(entry.to_string_lossy().to_string());
                }
            }
        }
    }

    if binaries_to_remove.is_empty() && backups_to_remove.is_empty() {
        println!("✓ No hal or halvor binaries or backup files found to remove.");
        println!("Checked locations:");
        for loc in &locations {
            println!("  - {}", loc);
        }
        return Ok(());
    }

    // Show what will be removed
    println!("The following files will be removed:");
    println!();
    if !binaries_to_remove.is_empty() {
        println!("Binaries:");
        for bin in &binaries_to_remove {
            println!("  - {}", bin);
        }
        println!();
    }
    if !backups_to_remove.is_empty() {
        println!("Backup files:");
        for backup in &backups_to_remove {
            println!("  - {}", backup);
        }
        println!();
    }

    // Ask for confirmation
    print!("Do you want to remove halvor binaries? [y/N]: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let remove_binaries =
        input.trim().eq_ignore_ascii_case("y") || input.trim().eq_ignore_ascii_case("yes");

    if !remove_binaries {
        println!("Skipping binary removal.");
        println!();
    }

    if remove_binaries {
        println!();
        println!("Removing binaries...");

        // Remove binaries
        for bin_path in &binaries_to_remove {
            let path = Path::new(bin_path);
            if path.exists() {
                if bin_path.starts_with("/usr") {
                    // System path, need sudo
                    println!("  Removing {} (requires sudo)...", bin_path);
                    let output = std::process::Command::new("sudo")
                        .arg("rm")
                        .arg("-f")
                        .arg(bin_path)
                        .output()?;
                    if !output.status.success() {
                        eprintln!("  ⚠ Warning: Failed to remove {}", bin_path);
                    } else {
                        println!("  ✓ Removed {}", bin_path);
                    }
                } else {
                    // User path, no sudo needed
                    println!("  Removing {}...", bin_path);
                    if let Err(e) = std::fs::remove_file(bin_path) {
                        eprintln!("  ⚠ Warning: Failed to remove {}: {}", bin_path, e);
                    } else {
                        println!("  ✓ Removed {}", bin_path);
                    }
                }
            }
        }

        // Remove backup files
        for backup_path in &backups_to_remove {
            let path = Path::new(backup_path);
            if path.exists() {
                if backup_path.starts_with("/usr") {
                    // System path, need sudo
                    println!("  Removing {} (requires sudo)...", backup_path);
                    let output = std::process::Command::new("sudo")
                        .arg("rm")
                        .arg("-f")
                        .arg(backup_path)
                        .output()?;
                    if !output.status.success() {
                        eprintln!("  ⚠ Warning: Failed to remove {}", backup_path);
                    } else {
                        println!("  ✓ Removed {}", backup_path);
                    }
                } else {
                    // User path, no sudo needed
                    println!("  Removing {}...", backup_path);
                    if let Err(e) = std::fs::remove_file(backup_path) {
                        eprintln!("  ⚠ Warning: Failed to remove {}: {}", backup_path, e);
                    } else {
                        println!("  ✓ Removed {}", backup_path);
                    }
                }
            }
        }

        println!();
        println!("✓ Binary removal complete");
        println!();
    }

    // Ask about removing database
    print!("Do you want to delete the halvor database? [y/N]: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let remove_database =
        input.trim().eq_ignore_ascii_case("y") || input.trim().eq_ignore_ascii_case("yes");

    if remove_database {
        println!();
        println!("Removing database...");

        if let Ok(db_path) = db::get_db_path() {
            if db_path.exists() {
                println!("  Database location: {}", db_path.display());
                println!("  Removing database...");
                if let Err(e) = std::fs::remove_file(&db_path) {
                    eprintln!("  ⚠ Warning: Failed to remove database: {}", e);
                } else {
                    println!("  ✓ Removed database");
                }
            } else {
                println!("  ✓ No database found to remove");
            }
        }
    } else {
        println!("Database preserved.");
        if let Ok(db_path) = db::get_db_path() {
            println!("  Location: {}", db_path.display());
        }
    }

    println!();

    // Ask about removing config data
    print!("Do you want to delete halvor configuration files? [y/N]: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let remove_config =
        input.trim().eq_ignore_ascii_case("y") || input.trim().eq_ignore_ascii_case("yes");

    if remove_config {
        println!();
        println!("Removing configuration files...");

        // Remove config file
        if let Ok(config_path) = config_manager::get_config_file_path() {
            if config_path.exists() {
                println!("  Removing config file: {}", config_path.display());
                if let Err(e) = std::fs::remove_file(&config_path) {
                    eprintln!("  ⚠ Warning: Failed to remove config file: {}", e);
                } else {
                    println!("  ✓ Removed config file");
                }
            }
        }

        // Remove encryption key
        if let Ok(config_dir) = config_manager::get_config_dir() {
            let key_path = config_dir.join(".halvor_key");
            if key_path.exists() {
                println!("  Removing encryption key: {}", key_path.display());
                if let Err(e) = std::fs::remove_file(&key_path) {
                    eprintln!("  ⚠ Warning: Failed to remove encryption key: {}", e);
                } else {
                    println!("  ✓ Removed encryption key");
                }
            }

            // Try to remove the config directory if it's empty
            if let Ok(mut entries) = std::fs::read_dir(&config_dir) {
                if entries.next().is_none() {
                    if let Err(e) = std::fs::remove_dir(&config_dir) {
                        eprintln!("  ⚠ Warning: Failed to remove config directory: {}", e);
                    } else {
                        println!("  ✓ Removed config directory");
                    }
                }
            }
        }
    } else {
        println!("Configuration files preserved.");
        if let Ok(config_path) = config_manager::get_config_file_path() {
            println!("  Location: {}", config_path.display());
        }
    }

    println!();
    println!("✓ Uninstall complete!");
    Ok(())
}
