use crate::config::{config_manager, env_file};
use crate::db;
use crate::{
    config::{HostConfig, find_homelab_dir, load_env_config},
    services::{
        delete_host_config as delete_host_config_service, get_host_config, list_hosts,
        store_host_config,
    },
};
use anyhow::{Context, Result};
use std::path::PathBuf;

/// Set a host field value (legacy - use update_host_config instead)
pub fn set_host_field(hostname: &str, field: &str, value: &str) -> Result<()> {
    let mut config = get_host_config(hostname)?.unwrap_or_else(|| HostConfig {
        ip: None,
        hostname: None,
        tailscale: None,
        backup_path: None,
    });

    match field {
        "ip" => config.ip = Some(value.to_string()),
        "hostname" => config.hostname = Some(value.to_string()),
        "tailscale" => config.tailscale = Some(value.to_string()),
        "backup_path" => config.backup_path = Some(value.to_string()),
        _ => anyhow::bail!("Unknown field: {}", field),
    }

    store_host_config(hostname, &config)?;
    println!("✓ Updated {} for host '{}'", field, hostname);
    Ok(())
}

/// Update host configuration with a partial or complete HostConfig
/// Only fields that are Some() will be updated
pub fn update_host_config(hostname: &str, updates: &HostConfig) -> Result<()> {
    let mut config = get_host_config(hostname)?.unwrap_or_else(|| HostConfig {
        ip: None,
        hostname: None,
        tailscale: None,
        backup_path: None,
    });

    // Update only fields that are Some()
    if let Some(ref ip) = updates.ip {
        config.ip = Some(ip.clone());
    }
    if let Some(ref hostname_val) = updates.hostname {
        config.hostname = Some(hostname_val.clone());
    }
    if let Some(ref tailscale) = updates.tailscale {
        config.tailscale = Some(tailscale.clone());
    }
    if let Some(ref backup_path) = updates.backup_path {
        config.backup_path = Some(backup_path.clone());
    }

    store_host_config(hostname, &config)?;
    println!("✓ Updated host configuration for '{}'", hostname);
    Ok(())
}

/// Replace host configuration completely
pub fn replace_host_config(hostname: &str, config: &HostConfig) -> Result<()> {
    store_host_config(hostname, config)?;
    println!("✓ Replaced host configuration for '{}'", hostname);
    Ok(())
}

/// Show host configuration
pub fn show_host_config(hostname: &str) -> Result<()> {
    let config =
        get_host_config(hostname)?.with_context(|| format!("Host '{}' not found", hostname))?;

    println!("Host configuration for '{}':", hostname);
    if let Some(ref ip) = config.ip {
        println!("  IP: {}", ip);
    }
    if let Some(ref hostname_val) = config.hostname {
        println!("  Hostname: {}", hostname_val);
    }
    if let Some(ref tailscale) = config.tailscale {
        println!("  Tailscale: {}", tailscale);
    }
    if let Some(ref backup_path) = config.backup_path {
        println!("  Backup Path: {}", backup_path);
    }
    Ok(())
}

/// Commit host configuration from .env to database
pub fn commit_host_config_to_db(hostname: &str) -> Result<()> {
    let homelab_dir = find_homelab_dir()?;
    let env_config = load_env_config(&homelab_dir)?;

    if let Some(config) = env_config.hosts.get(hostname) {
        store_host_config(hostname, config)?;
        println!(
            "✓ Committed host configuration for '{}' from .env to database",
            hostname
        );
    } else {
        anyhow::bail!("Host '{}' not found in .env file", hostname);
    }
    Ok(())
}

/// Backup host configuration from database to .env
pub fn backup_host_config_to_env(hostname: &str) -> Result<()> {
    let config = get_host_config(hostname)?
        .with_context(|| format!("Host '{}' not found in database", hostname))?;

    let homelab_dir = find_homelab_dir()?;
    let env_path = homelab_dir.join(".env");

    env_file::write_host_to_env_file(&env_path, hostname, &config)?;
    println!(
        "✓ Backed up host configuration for '{}' from database to .env",
        hostname
    );
    Ok(())
}

/// Delete host configuration
pub fn delete_host_config(hostname: &str, from_env: bool) -> Result<()> {
    delete_host_config_service(hostname)?;
    println!(
        "✓ Deleted host configuration for '{}' from database",
        hostname
    );

    if from_env {
        let homelab_dir = find_homelab_dir()?;
        let env_path = homelab_dir.join(".env");
        env_file::remove_host_from_env_file(&env_path, hostname)?;
        println!(
            "✓ Removed host configuration for '{}' from .env file",
            hostname
        );
    }
    Ok(())
}

/// Commit all host configurations from .env to database
pub fn commit_all_to_db() -> Result<()> {
    let homelab_dir = find_homelab_dir()?;
    let env_config = load_env_config(&homelab_dir)?;

    let mut count = 0;
    for (hostname, config) in &env_config.hosts {
        store_host_config(hostname, config)?;
        count += 1;
    }

    println!(
        "✓ Committed {} host configuration(s) from .env to database",
        count
    );
    Ok(())
}

/// Backup all host configurations from database to .env
pub fn backup_all_to_env() -> Result<()> {
    let hosts = list_hosts()?;
    let homelab_dir = find_homelab_dir()?;
    let env_path = homelab_dir.join(".env");

    let mut count = 0;
    for hostname in &hosts {
        if let Some(config) = get_host_config(hostname)? {
            env_file::write_host_to_env_file(&env_path, hostname, &config)?;
            count += 1;
        }
    }

    println!(
        "✓ Backed up {} host configuration(s) from database to .env",
        count
    );
    Ok(())
}

/// Set backup location for a host
pub fn set_backup_location(hostname: Option<&str>) -> Result<()> {
    use std::io::{self, Write};

    let hostname = if let Some(h) = hostname {
        h.to_string()
    } else {
        print!("Enter hostname: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        input.trim().to_string()
    };

    print!("Enter backup path for {}: ", hostname);
    io::stdout().flush()?;
    let mut backup_path = String::new();
    io::stdin().read_line(&mut backup_path)?;
    let backup_path = backup_path.trim().to_string();

    set_host_field(&hostname, "backup_path", &backup_path)?;
    Ok(())
}

/// Show configuration from database
pub fn show_db_config(verbose: bool) -> Result<()> {
    let hosts = list_hosts()?;

    if hosts.is_empty() {
        println!("No hosts found in database.");
        return Ok(());
    }

    println!("Host configurations from database:");
    println!();

    for hostname in &hosts {
        if let Some(config) = get_host_config(hostname)? {
            println!("Host: {}", hostname);
            if verbose || config.ip.is_some() {
                if let Some(ref ip) = config.ip {
                    println!("  IP: {}", ip);
                }
            }
            if verbose || config.hostname.is_some() {
                if let Some(ref hostname_val) = config.hostname {
                    println!("  Hostname: {}", hostname_val);
                }
            }
            if verbose || config.tailscale.is_some() {
                if let Some(ref tailscale) = config.tailscale {
                    println!("  Tailscale: {}", tailscale);
                }
            }
            if verbose || config.backup_path.is_some() {
                if let Some(ref backup_path) = config.backup_path {
                    println!("  Backup Path: {}", backup_path);
                }
            }
            println!();
        }
    }
    Ok(())
}

/// Show current configuration from .env
pub fn show_current_config(verbose: bool) -> Result<()> {
    let homelab_dir = find_homelab_dir()?;
    let env_config = load_env_config(&homelab_dir)?;

    if env_config.hosts.is_empty() {
        println!("No hosts found in .env file.");
        return Ok(());
    }

    println!("Host configurations from .env:");
    println!();

    let mut hostnames: Vec<_> = env_config.hosts.keys().collect();
    hostnames.sort();

    for hostname in hostnames {
        let config = &env_config.hosts[hostname];
        println!("Host: {}", hostname);
        if verbose || config.ip.is_some() {
            if let Some(ref ip) = config.ip {
                println!("  IP: {}", ip);
            }
        }
        if verbose || config.hostname.is_some() {
            if let Some(ref hostname_val) = config.hostname {
                println!("  Hostname: {}", hostname_val);
            }
        }
        if verbose || config.tailscale.is_some() {
            if let Some(ref tailscale) = config.tailscale {
                println!("  Tailscale: {}", tailscale);
            }
        }
        if verbose || config.backup_path.is_some() {
            if let Some(ref backup_path) = config.backup_path {
                println!("  Backup Path: {}", backup_path);
            }
        }
        println!();
    }
    Ok(())
}

/// Set environment file path
pub fn set_env_path(path: &str) -> Result<()> {
    config_manager::set_env_file_path(PathBuf::from(path).as_path())
}

/// Create example .env file
pub fn create_example_env_file() -> Result<()> {
    let homelab_dir = find_homelab_dir()?;
    let env_path = homelab_dir.join(".env.example");

    let example_content = r#"# HAL Configuration
# Copy this file to .env and fill in your values

# Tailnet base domain (e.g., ts.net)
TAILNET_BASE=ts.net

# Host configurations
# Format: HOST_<HOSTNAME>_<FIELD>=<value>
# Example:
# HOST_bellerophon_IP=192.168.1.100
# HOST_bellerophon_HOSTNAME=bellerophon
# HOST_bellerophon_TAILSCALE=bellerophon
# HOST_bellerophon_BACKUP_PATH=/mnt/backups/bellerophon

# SMB Server configurations
# Format: SMB_<SERVERNAME>_<FIELD>=<value>
# Example:
# SMB_nas_HOST=192.168.1.50
# SMB_nas_SHARES=media,backups
# SMB_nas_USERNAME=user
# SMB_nas_PASSWORD=password

# Nginx Proxy Manager
# NPM_URL=https://npm.example.com:81
# NPM_USERNAME=admin
# NPM_PASSWORD=changeme
"#;

    std::fs::write(&env_path, example_content)
        .with_context(|| format!("Failed to write example .env file: {}", env_path.display()))?;

    println!("✓ Created example .env file at {}", env_path.display());
    Ok(())
}

/// Backup SQLite database
pub fn backup_database(path: Option<&str>) -> Result<()> {
    use chrono::Utc;
    use std::fs;

    let db_path = db::get_db_path()?;

    if !db_path.exists() {
        anyhow::bail!("Database not found at {}", db_path.display());
    }

    let backup_path = if let Some(p) = path {
        PathBuf::from(p)
    } else {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        std::env::current_dir()?.join(format!("halvor_backup_{}.db", timestamp))
    };

    // Use sudo to ensure we have proper access to the database
    // This ensures we can read the database even if it has restricted permissions
    #[cfg(unix)]
    {
        // Try to copy with sudo first (for system-protected databases)
        let sudo_copy = std::process::Command::new("sudo")
            .arg("cp")
            .arg(&db_path)
            .arg(&backup_path)
            .output();

        if let Ok(output) = sudo_copy {
            if output.status.success() {
                println!("✓ Database backed up to {}", backup_path.display());
                println!("  Note: Backup is unencrypted (plain SQLite format)");
                return Ok(());
            }
        }
    }

    // Fallback to regular copy (for user-owned databases)
    fs::copy(&db_path, &backup_path).with_context(|| {
        format!(
            "Failed to copy database from {} to {}. You may need administrator privileges.",
            db_path.display(),
            backup_path.display()
        )
    })?;

    println!("✓ Database backed up to {}", backup_path.display());
    println!("  Note: Backup is unencrypted (plain SQLite format)");
    Ok(())
}

/// Show differences between .env and database configurations
pub fn show_config_diff() -> Result<()> {
    let homelab_dir = find_homelab_dir()?;
    let env_config = load_env_config(&homelab_dir)?;
    let db_hosts = list_hosts().unwrap_or_default();

    let mut env_hostnames: Vec<_> = env_config.hosts.keys().collect();
    env_hostnames.sort();

    let mut all_hostnames = std::collections::HashSet::new();
    for hostname in &env_hostnames {
        all_hostnames.insert(*hostname);
    }
    for hostname in &db_hosts {
        all_hostnames.insert(hostname);
    }

    let mut all_hostnames: Vec<_> = all_hostnames.into_iter().collect();
    all_hostnames.sort();

    if all_hostnames.is_empty() {
        println!("No hosts found in either .env or database.");
        return Ok(());
    }

    println!("Configuration differences between .env and database:");
    println!();

    for hostname in &all_hostnames {
        let env_config = env_config.hosts.get(*hostname);
        let db_config = get_host_config(hostname).ok().flatten();

        match (env_config, db_config) {
            (Some(env), Some(db)) => {
                // Compare fields
                let mut has_diff = false;
                if env.ip != db.ip {
                    println!("  {} - IP differs:", hostname);
                    if let Some(ref ip) = env.ip {
                        println!("    .env: {}", ip);
                    } else {
                        println!("    .env: (not set)");
                    }
                    if let Some(ref ip) = db.ip {
                        println!("    db:   {}", ip);
                    } else {
                        println!("    db:   (not set)");
                    }
                    has_diff = true;
                }
                if env.hostname != db.hostname {
                    println!("  {} - Hostname differs:", hostname);
                    if let Some(ref h) = env.hostname {
                        println!("    .env: {}", h);
                    } else {
                        println!("    .env: (not set)");
                    }
                    if let Some(ref h) = db.hostname {
                        println!("    db:   {}", h);
                    } else {
                        println!("    db:   (not set)");
                    }
                    has_diff = true;
                }
                if env.tailscale != db.tailscale {
                    println!("  {} - Tailscale differs:", hostname);
                    if let Some(ref t) = env.tailscale {
                        println!("    .env: {}", t);
                    } else {
                        println!("    .env: (not set)");
                    }
                    if let Some(ref t) = db.tailscale {
                        println!("    db:   {}", t);
                    } else {
                        println!("    db:   (not set)");
                    }
                    has_diff = true;
                }
                if env.backup_path != db.backup_path {
                    println!("  {} - Backup path differs:", hostname);
                    if let Some(ref p) = env.backup_path {
                        println!("    .env: {}", p);
                    } else {
                        println!("    .env: (not set)");
                    }
                    if let Some(ref p) = db.backup_path {
                        println!("    db:   {}", p);
                    } else {
                        println!("    db:   (not set)");
                    }
                    has_diff = true;
                }
                if !has_diff {
                    println!("  {} - No differences", hostname);
                }
            }
            (Some(_), None) => {
                println!("  {} - Only in .env (not in database)", hostname);
            }
            (None, Some(_)) => {
                println!("  {} - Only in database (not in .env)", hostname);
            }
            (None, None) => {
                // Shouldn't happen, but handle it
            }
        }
        println!();
    }

    Ok(())
}
