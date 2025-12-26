use crate::config::{config_manager, env_file};
use crate::db;
use crate::{
    config::{EnvConfig, HostConfig, find_halvor_dir, load_env_config},
    services::{
        delete_host_config as delete_host_config_service, get_host_config, list_hosts,
        store_host_config,
    },
};
use anyhow::{Context, Result};
use std::io::{self, Write};
use std::path::PathBuf;

/// Set a host field value (legacy - use update_host_config instead)
pub fn set_host_field(hostname: &str, field: &str, value: &str) -> Result<()> {
    let mut config = get_host_config(hostname)?.unwrap_or_else(|| HostConfig {
        ip: None,
        hostname: None,
        backup_path: None,
        sudo_password: None,
        sudo_user: None,
    });

    match field {
        "ip" => config.ip = Some(value.to_string()),
        "hostname" => config.hostname = Some(value.to_string()),
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
        backup_path: None,
        sudo_password: None,
        sudo_user: None,
    });

    // Update only fields that are Some()
    if let Some(ref ip) = updates.ip {
        config.ip = Some(ip.clone());
    }
    if let Some(ref hostname_val) = updates.hostname {
        config.hostname = Some(hostname_val.clone());
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
    // Hostname is shown above, no separate tailscale field
    if let Some(ref backup_path) = config.backup_path {
        println!("  Backup Path: {}", backup_path);
    }
    Ok(())
}

/// Commit host configuration (no-op - config is only in .env)
pub fn commit_host_config_to_db(hostname: &str) -> Result<()> {
    let halvor_dir = find_halvor_dir()?;
    let env_config = load_env_config(&halvor_dir)?;

    if env_config.hosts.get(hostname).is_some() {
        // Config is already in .env (loaded from 1Password)
        println!(
            "✓ Host configuration for '{}' is in .env file (loaded from 1Password)",
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

    let halvor_dir = find_halvor_dir()?;
    let env_path = halvor_dir.join(".env");

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
        let halvor_dir = find_halvor_dir()?;
        let env_path = halvor_dir.join(".env");
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
    let halvor_dir = find_halvor_dir()?;
    let env_config = load_env_config(&halvor_dir)?;

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

/// Backup all host configurations from database to .env (with .env backup first)
pub fn backup_all_to_env_with_backup() -> Result<()> {
    use chrono::Utc;
    use std::fs;

    let halvor_dir = find_halvor_dir()?;
    let env_path = halvor_dir.join(".env");

    // Backup current .env file if it exists
    if env_path.exists() {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let backup_path = halvor_dir.join(format!(".env.backup_{}", timestamp));
        fs::copy(&env_path, &backup_path)
            .with_context(|| format!("Failed to backup .env file to {}", backup_path.display()))?;
        println!("✓ Backed up current .env to {}", backup_path.display());
    }

    // Now write all DB configs to .env
    let hosts = list_hosts()?;
    let mut count = 0;
    for hostname in &hosts {
        if let Some(config) = get_host_config(hostname)? {
            env_file::write_host_to_env_file(&env_path, hostname, &config)?;
            count += 1;
        }
    }

    println!(
        "✓ Wrote {} host configuration(s) from database to .env",
        count
    );
    Ok(())
}

/// Backup all host configurations from database to .env (legacy, no backup)
pub fn backup_all_to_env() -> Result<()> {
    backup_all_to_env_with_backup()
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
            // Hostname is shown above, no separate tailscale field
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

/// Show current configuration from .env (loaded from 1Password)
pub fn show_current_config(verbose: bool) -> Result<()> {
    use std::env;
    let halvor_dir = find_halvor_dir()?;
    let env_config = load_env_config(&halvor_dir)?;
    // Only use .env config (loaded from 1Password via .envrc)

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Configuration (.env file)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Show Tailnet configuration (env vs db)
    println!("Tailnet:");
    let env_tld = env::var("TAILNET_TLD").or_else(|_| env::var("TLD")).ok();
    let env_acme = env::var("ACME_EMAIL").ok();

    println!("  Base: {}", env_config._tailnet_base);
    if let Some(tld) = env_tld {
        println!("  TLD: {}", tld);
    }
    if let Some(acme) = env_acme {
        println!("  ACME Email: {}", acme);
    }
    println!();

    // Show PIA VPN configuration (from .env)
    println!("PIA VPN:");
    let pia_username = env::var("PIA_USERNAME").ok();
    let pia_password = env::var("PIA_PASSWORD").ok();
    if pia_username.is_some() || pia_password.is_some() {
        if let Some(ref username) = pia_username {
            println!("  Username: {}", username);
        } else {
            println!("  Username: (not set)");
        }
        if verbose {
            if let Some(ref password) = pia_password {
                println!("  Password: {}", password);
            } else {
                println!("  Password: (not set)");
            }
        } else {
            println!(
                "  Password: {}",
                if pia_password.is_some() {
                    "***"
                } else {
                    "(not set)"
                }
            );
        }
    } else {
        println!("  (not configured)");
    }
    println!();

    // Show media paths (from .env)
    println!("Media Paths:");
    let paths = [
        ("Downloads", "DOWNLOADS_PATH"),
        ("Movies", "MOVIES_PATH"),
        ("TV", "TV_PATH"),
        ("Movies 4K", "MOVIES_4K_PATH"),
        ("Music", "MUSIC_PATH"),
    ];
    let mut has_paths = false;
    for (name, var) in &paths {
        if let Ok(path) = env::var(var) {
            has_paths = true;
            println!("  {}: {}", name, path);
        }
    }
    if !has_paths {
        println!("  (not configured)");
    }
    println!();

    // Show NPM configuration (from .env)
    println!("Nginx Proxy Manager:");
    let npm_url = crate::config::get_npm_url();
    let npm_username = crate::config::get_npm_username();
    let npm_password = crate::config::get_npm_password();
    if npm_url.is_some() || npm_username.is_some() || npm_password.is_some() {
        if let Some(ref url) = npm_url {
            println!("  URL: {}", url);
        }
        if let Some(ref username) = npm_username {
            println!("  Username: {}", username);
        }
        if verbose {
            if let Some(ref password) = npm_password {
                println!("  Password: {}", password);
            } else {
                println!("  Password: (not set)");
            }
        } else {
            println!(
                "  Password: {}",
                if npm_password.is_some() {
                    "***"
                } else {
                    "(not set)"
                }
            );
        }
    } else {
        println!("  (not configured)");
    }
    println!();

    // Show SMB servers (env vs db)
    println!("SMB Servers:");
    let mut server_names: Vec<String> = env_config.smb_servers.keys().cloned().collect();
    server_names.sort();
    if server_names.is_empty() {
        println!("  (none configured)");
    }
    for server_name in server_names {
        let env_cfg = env_config.smb_servers.get(&server_name);
        println!("  {}:", server_name);
        let host_line = |label: &str, env_val: Option<String>| {
            if let Some(ev) = env_val {
                println!("    {}: {}", label, ev);
            }
        };
        if let Some(cfg) = env_cfg {
            host_line("Host", Some(cfg.host.clone()));
            host_line("Shares", Some(cfg.shares.join(", ")));
            host_line("Username", cfg.username.clone());
            if verbose {
                host_line("Password", cfg.password.clone());
            } else {
                let mask = cfg.password.as_ref().map(|_| "***".to_string());
                host_line("Password", mask);
            }
            host_line("Options", cfg.options.clone());
        }
    }
    println!();

    // Show hosts (from .env file loaded from 1Password)
    println!("Hosts:");
    let mut hostnames: Vec<String> = env_config.hosts.keys().cloned().collect();
    hostnames.sort();
    if hostnames.is_empty() {
        println!("  (none configured)");
        println!();
    } else {
        for hostname in hostnames {
            let cfg = env_config.hosts.get(&hostname).unwrap();
            println!("  {}:", hostname);
            if let Some(ip) = &cfg.ip {
                println!("    IP: {}", ip);
            }
            if let Some(hostname_val) = &cfg.hostname {
                println!("    Hostname: {}", hostname_val);
            }
            if let Some(backup_path) = &cfg.backup_path {
                println!("    Backup Path: {}", backup_path);
            }
        }
        println!();
    }

    // Show any other env values that were not explicitly printed above
    let known_vars = [
        "TAILNET_BASE",
        "TAILNET_TLD",
        "TLD",
        "ACME_EMAIL",
        "PIA_USERNAME",
        "PIA_PASSWORD",
        "DOWNLOADS_PATH",
        "MOVIES_PATH",
        "TV_PATH",
        "MOVIES_4K_PATH",
        "MUSIC_PATH",
        "NGINX_PROXY_MANAGER_URL",
        "NGINX_PROXY_MANAGER_USERNAME",
        "NGINX_PROXY_MANAGER_PASSWORD",
    ];

    let env_snapshot: Vec<(String, String)> = env::vars().collect();
    let mut other_vars: Vec<(String, String)> = env_snapshot
        .into_iter()
        .filter(|(k, _)| {
            // skip host/smb/NPM/PIA/etc keys already displayed
            !(k.starts_with("HOST_") || k.starts_with("SMB_") || known_vars.contains(&k.as_str()))
        })
        .collect();
    other_vars.sort_by(|a, b| a.0.cmp(&b.0));

    if verbose && !other_vars.is_empty() {
        println!("Other environment values:");
        for (k, v) in other_vars {
            // Mask passwords by simple heuristic
            let masked =
                if k.to_lowercase().contains("password") || k.to_lowercase().contains("secret") {
                    "***".to_string()
                } else {
                    v
                };
            println!("  {}={}", k, masked);
        }
        println!();
    }

    // Show validation status
    println!("Validation:");
    let mut valid = true;
    let mut issues = Vec::new();

    if env_config.hosts.is_empty() {
        issues.push("No hosts configured".to_string());
        valid = false;
    }

    for (name, server) in &env_config.smb_servers {
        if server.host.is_empty() {
            issues.push(format!("SMB server '{}' missing host", name));
            valid = false;
        }
        if server.shares.is_empty() {
            issues.push(format!("SMB server '{}' missing shares", name));
            valid = false;
        }
    }

    if valid {
        println!("  ✓ Configuration is valid");
    } else {
        println!("  ✗ Configuration has issues:");
        for issue in issues {
            println!("    - {}", issue);
        }
    }
    println!();

    Ok(())
}

/// Set environment file path
pub fn set_env_path(path: &str) -> Result<()> {
    config_manager::set_env_file_path(PathBuf::from(path).as_path())
}

/// Create example .env file
pub fn create_example_env_file() -> Result<()> {
    let halvor_dir = find_halvor_dir()?;
    let env_path = halvor_dir.join(".env.example");

    let example_content = r#"# HAL Configuration
# Copy this file to .env and fill in your values

# Tailnet base domain (e.g., ts.net)
TAILNET_BASE=ts.net

# Host configurations
# Format: HOST_<HOSTNAME>_<FIELD>=<value>
# Example:
# HOST_bellerophon_IP=192.168.1.100
# HOST_bellerophon_HOSTNAME=bellerophon
# HOST_bellerophon_HOSTNAME=bellerophon
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

/// Show configuration (from .env file loaded from 1Password)
/// Note: Database comparison removed - only .env is used
pub fn show_config_diff() -> Result<()> {
    let halvor_dir = find_halvor_dir()?;
    let env_config = load_env_config(&halvor_dir)?;

    if env_config.hosts.is_empty() {
        println!("No hosts found in .env file.");
        return Ok(());
    }

    println!("Hosts in .env file (loaded from 1Password):");
    println!();

    let mut hostnames: Vec<_> = env_config.hosts.keys().collect();
    hostnames.sort();

    for hostname in &hostnames {
        let cfg = env_config.hosts.get(*hostname).unwrap();
        println!("  {}:", hostname);
        if let Some(ip) = &cfg.ip {
            println!("    IP: {}", ip);
        }
        if let Some(hostname_val) = &cfg.hostname {
            println!("    Hostname: {}", hostname_val);
        }
        if let Some(backup_path) = &cfg.backup_path {
            println!("    Backup Path: {}", backup_path);
        }
        println!();
    }

    Ok(())
}

/// Get the current machine's hostname
pub fn get_current_hostname() -> Result<String> {
    use crate::utils::exec::local;
    use std::env;

    // Try multiple methods to get hostname
    // 1. Try HOSTNAME environment variable
    if let Ok(hostname) = env::var("HOSTNAME") {
        if !hostname.is_empty() {
            return Ok(hostname.trim().to_string());
        }
    }

    // 2. Try hostname command
    if let Ok(output) = local::execute("hostname", &[]) {
        if output.status.success() {
            let hostname = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !hostname.is_empty() {
                return Ok(hostname);
            }
        }
    }

    // 3. Try /etc/hostname (Unix)
    #[cfg(unix)]
    {
        if let Ok(hostname) = std::fs::read_to_string("/etc/hostname") {
            let hostname = hostname.trim().to_string();
            if !hostname.is_empty() {
                return Ok(hostname);
            }
        }
    }

    // 4. Fallback to COMPUTERNAME on Windows
    #[cfg(windows)]
    {
        if let Ok(hostname) = env::var("COMPUTERNAME") {
            if !hostname.is_empty() {
                return Ok(hostname.trim().to_string());
            }
        }
    }

    anyhow::bail!("Could not determine hostname")
}

/// Normalize hostname by stripping TLDs to find base hostname
pub fn normalize_hostname(hostname: &str) -> String {
    // Common TLDs to strip
    let tlds = [".scottkey.me", ".ts.net", ".local", ".lan"];

    let mut normalized = hostname.to_string();
    for tld in &tlds {
        if normalized.ends_with(tld) {
            normalized = normalized[..normalized.len() - tld.len()].to_string();
            break;
        }
    }

    // Also try stripping any domain (everything after first dot if it looks like a domain)
    if normalized.contains('.')
        && !normalized.starts_with("127.")
        && !normalized.starts_with("192.168.")
        && !normalized.starts_with("10.")
    {
        if let Some(first_dot) = normalized.find('.') {
            // Check if the part after the dot looks like a TLD (short, no numbers)
            let after_dot = &normalized[first_dot + 1..];
            if after_dot.len() <= 10 && !after_dot.chars().any(|c| c.is_ascii_digit()) {
                normalized = normalized[..first_dot].to_string();
            }
        }
    }

    normalized
}

/// Find hostname in config, trying normalized versions if exact match fails
pub fn find_hostname_in_config(hostname: &str, config: &EnvConfig) -> Option<String> {
    // Try exact match first
    if config.hosts.contains_key(hostname) {
        return Some(hostname.to_string());
    }

    // Try normalized version (strip TLDs)
    let normalized = normalize_hostname(hostname);
    if normalized != hostname && config.hosts.contains_key(&normalized) {
        return Some(normalized);
    }

    // Try case-insensitive match
    for (key, _) in &config.hosts {
        if key.eq_ignore_ascii_case(hostname) {
            return Some(key.clone());
        }
        if key.eq_ignore_ascii_case(&normalized) {
            return Some(key.clone());
        }
    }

    None
}

/// Ensure hostname is in config, prompt to set it up if not
/// Returns the hostname to use (may be different from input if user chose to set up current machine)
pub fn ensure_host_in_config(hostname: Option<&str>, config: &EnvConfig) -> Result<String> {
    // If hostname is provided, check if it exists (try normalized versions)
    if let Some(host) = hostname {
        if let Some(found_host) = find_hostname_in_config(host, config) {
            return Ok(found_host);
        }
        // Hostname provided but not found - return error with helpful message
        anyhow::bail!(
            "Host '{}' not found in config.\n\nAdd to .env:\n  HOST_{}_IP=\"<ip-address>\"\n  HOST_{}_HOSTNAME=\"<hostname>\"",
            host,
            host.to_uppercase(),
            host.to_uppercase()
        );
    }

    // No hostname provided - detect current machine
    let detected_hostname = get_current_hostname()?;

    // Check if current machine is in config
    if config.hosts.contains_key(&detected_hostname) {
        return Ok(detected_hostname);
    }

    // Current machine not in config - prompt to set it up
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(
        "Current machine '{}' not found in configuration",
        detected_hostname
    );
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Each system running halvor becomes a 'node' in your homelab.");
    println!("Would you like to set up this machine as a node?");
    println!();
    print!("Set up '{}' as a node? [Y/n]: ", detected_hostname);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let response = input.trim().to_lowercase();

    if !response.is_empty() && response != "y" && response != "yes" {
        anyhow::bail!("Setup cancelled. Add host configuration to .env file manually.");
    }

    // Interactive setup
    println!();
    println!("Setting up node...");
    println!();

    // Configure hostname (allow override)
    print!("Hostname [{}]: ", detected_hostname);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let current_hostname = input.trim();
    let current_hostname = if current_hostname.is_empty() {
        detected_hostname
    } else {
        current_hostname.to_string()
    };

    #[cfg(debug_assertions)]
    println!("[DEBUG] Using hostname: {}", current_hostname);

    // Get IP address - auto-detect
    use crate::utils::networking;
    let local_ips = networking::get_local_ips()?;
    let ip = if local_ips.is_empty() {
        // No IPs detected - prompt user
        print!("Enter IP address: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let ip = input.trim().to_string();
        if ip.is_empty() {
            anyhow::bail!("IP address is required");
        }
        ip
    } else if local_ips.len() == 1 {
        // Single IP detected - use it automatically
        println!("✓ Detected IP: {}", local_ips[0]);
        local_ips[0].clone()
    } else {
        // Multiple IPs detected - prefer non-loopback, non-link-local
        let preferred_ips: Vec<_> = local_ips
            .iter()
            .filter(|ip| {
                !ip.starts_with("127.")
                    && !ip.starts_with("169.254.")
                    && !ip.starts_with("fe80:")
                    && !ip.starts_with("::1")
            })
            .collect();

        if preferred_ips.len() == 1 {
            // One preferred IP - use it automatically
            println!("✓ Detected IP: {}", preferred_ips[0]);
            preferred_ips[0].to_string()
        } else if !preferred_ips.is_empty() {
            // Multiple preferred IPs - show them and let user choose
            println!("Multiple IP addresses detected:");
            for (i, ip) in preferred_ips.iter().enumerate() {
                println!("  [{}] {}", i + 1, ip);
            }
            print!("Select IP address [1]: ");
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let selection = input.trim();
            if selection.is_empty() {
                preferred_ips[0].to_string()
            } else {
                let idx: usize = selection.parse().with_context(|| "Invalid selection")?;
                if idx < 1 || idx > preferred_ips.len() {
                    anyhow::bail!("Invalid selection");
                }
                preferred_ips[idx - 1].to_string()
            }
        } else {
            // Only loopback/link-local IPs - show all and let user choose
            println!("Only loopback/link-local IPs detected:");
            for (i, ip) in local_ips.iter().enumerate() {
                println!("  [{}] {}", i + 1, ip);
            }
            print!("Select IP address [1]: ");
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let selection = input.trim();
            if selection.is_empty() {
                local_ips[0].clone()
            } else {
                let idx: usize = selection.parse().with_context(|| "Invalid selection")?;
                if idx < 1 || idx > local_ips.len() {
                    anyhow::bail!("Invalid selection");
                }
                local_ips[idx - 1].clone()
            }
        }
    };

    let tailscale_ips = networking::get_tailscale_ips().ok().unwrap_or_default();

    // Get Tailscale hostname (optional)
    use crate::services::tailscale;
    let tailscale_hostname = tailscale::get_tailscale_hostname().ok().flatten();
    let tailscale = if let Some(ts) = tailscale_hostname {
        println!("Detected Tailscale hostname: {}", ts);
        print!("Use this Tailscale hostname? [Y/n]: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let response = input.trim().to_lowercase();
        if response.is_empty() || response == "y" || response == "yes" {
            Some(ts)
        } else {
            print!("Enter Tailscale hostname (or press Enter to skip): ");
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let ts = input.trim();
            if ts.is_empty() {
                None
            } else {
                Some(ts.to_string())
            }
        }
    } else {
        print!("Enter Tailscale hostname (or press Enter to skip): ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let ts = input.trim();
        if ts.is_empty() {
            None
        } else {
            Some(ts.to_string())
        }
    };

    // Get Tailscale IP (optional) - can be used as primary IP
    // Use detected Tailscale IPs from networking module, or fallback to tailscale service
    let use_tailscale_ip = if !tailscale_ips.is_empty() {
        if tailscale_ips.len() == 1 {
            println!("Detected Tailscale IP: {}", tailscale_ips[0]);
            print!("Use Tailscale IP as primary IP? [y/N]: ");
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let response = input.trim().to_lowercase();
            if response == "y" || response == "yes" {
                Some(tailscale_ips[0].clone())
            } else {
                None
            }
        } else {
            // Multiple Tailscale IPs - let user choose
            println!("Multiple Tailscale IPs detected:");
            for (i, ts_ip) in tailscale_ips.iter().enumerate() {
                println!("  [{}] {}", i + 1, ts_ip);
            }
            print!("Select Tailscale IP to use as primary (or press Enter to skip) [1]: ");
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let selection = input.trim();
            if selection.is_empty() {
                None
            } else {
                let idx: usize = selection.parse().with_context(|| "Invalid selection")?;
                if idx >= 1 && idx <= tailscale_ips.len() {
                    Some(tailscale_ips[idx - 1].clone())
                } else {
                    None
                }
            }
        }
    } else {
        // Fallback to tailscale service detection
        let tailscale_ip = tailscale::get_tailscale_ip().ok().flatten();
        if let Some(ts_ip) = tailscale_ip {
            println!("Detected Tailscale IP: {}", ts_ip);
            print!("Use Tailscale IP as primary IP? [y/N]: ");
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let response = input.trim().to_lowercase();
            if response == "y" || response == "yes" {
                Some(ts_ip)
            } else {
                None
            }
        } else {
            print!("Enter Tailscale IP to use as primary IP (or press Enter to skip): ");
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let ts_ip = input.trim();
            if ts_ip.is_empty() {
                None
            } else {
                Some(ts_ip.to_string())
            }
        }
    };

    // Use Tailscale IP as primary IP if provided, otherwise use detected IP
    let using_tailscale_ip = use_tailscale_ip.is_some();
    let final_ip = use_tailscale_ip.unwrap_or(ip);

    #[cfg(debug_assertions)]
    println!("[DEBUG] Final configuration:");
    #[cfg(debug_assertions)]
    println!("[DEBUG]   hostname: {}", current_hostname);
    #[cfg(debug_assertions)]
    println!("[DEBUG]   ip: {}", final_ip);
    #[cfg(debug_assertions)]
    println!("[DEBUG]   tailscale hostname: {:?}", tailscale);
    if using_tailscale_ip {
        #[cfg(debug_assertions)]
        println!("[DEBUG]   using Tailscale IP as primary IP");
    }

    // Create host config
    // Note: tailscale variable is now stored in hostname field
    let host_config = HostConfig {
        ip: Some(final_ip),
        hostname: tailscale, // Use tailscale hostname as the hostname field
        backup_path: None,
        sudo_password: None,
        sudo_user: None,
    };

    // Store to .env file (loaded from 1Password)
    #[cfg(debug_assertions)]
    println!("[DEBUG] Storing host config to .env file:");
    #[cfg(debug_assertions)]
    println!("[DEBUG]   hostname: {}", current_hostname);
    #[cfg(debug_assertions)]
    println!("[DEBUG]   ip: {:?}", host_config.ip);
    #[cfg(debug_assertions)]
    println!("[DEBUG]   hostname field: {:?}", host_config.hostname);
    // Hostname field covers both hostname and tailscale
    store_host_config(&current_hostname, &host_config).with_context(|| {
        format!(
            "Failed to store host config for '{}' to .env file",
            current_hostname
        )
    })?;

    #[cfg(debug_assertions)]
    println!("[DEBUG] ✓ Host config stored to database");

    // Verify it can be retrieved
    #[cfg(debug_assertions)]
    {
        match get_host_config(&current_hostname) {
            Ok(Some(retrieved)) => {
                println!("[DEBUG] ✓ Verified: Host config retrieved from database");
                println!("[DEBUG]   Retrieved hostname: {:?}", retrieved.hostname);
                println!("[DEBUG]   Retrieved IP: {:?}", retrieved.ip);
                // Hostname field covers both hostname and tailscale
            }
            Ok(None) => {
                eprintln!("[DEBUG] ⚠ Warning: Host config not found after storing");
            }
            Err(e) => {
                eprintln!("[DEBUG] ⚠ Error retrieving host config: {}", e);
            }
        }
    }

    println!();
    println!("✓ Node '{}' configured successfully!", current_hostname);
    println!("  Configuration saved to database");
    println!();

    Ok(current_hostname)
}

/// Handle create config commands
pub fn handle_create_config(command: crate::commands::config::CreateConfigCommands) -> Result<()> {
    match command {
        crate::commands::config::CreateConfigCommands::App => {
            println!(
                "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            );
            println!("Create App Configuration");
            println!(
                "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            );
            println!();
            println!("⚠ App configuration creation not yet implemented");
        }
        crate::commands::config::CreateConfigCommands::Smb { server_name: _ } => {
            println!(
                "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            );
            println!("Create SMB Server Configuration");
            println!(
                "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            );
            println!();
            println!("⚠ SMB configuration creation not yet implemented");
        }
        crate::commands::config::CreateConfigCommands::Ssh { hostname: _ } => {
            println!(
                "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            );
            println!("Create SSH Host Configuration");
            println!(
                "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            );
            println!();
            println!("⚠ SSH configuration creation not yet implemented");
        }
    }
    Ok(())
}

/// Handle config command routing and dispatch
pub fn handle_config_command(
    arg: Option<&str>,
    verbose: bool,
    db: bool,
    command: Option<&crate::commands::config::ConfigCommands>,
) -> Result<()> {
    use crate::commands::config::ConfigCommands;

    // Known global commands that should not be treated as hostnames
    let global_commands = [
        "list",
        "init",
        "set-env",
        "stable",
        "experimental",
        "create",
        "env",
        "db",
        "backup",
        "commit",
        "delete",
        "diff",
        "kubeconfig",
        "regenerate",
    ];

    // If arg is provided and it's not a known command, treat it as a hostname
    if let Some(arg_str) = arg {
        if !global_commands.contains(&arg_str.to_lowercase().as_str()) {
            // This is a hostname
            let hostname = arg_str;
            match command {
                None | Some(ConfigCommands::List) => {
                    show_host_config(hostname)?;
                }
                Some(ConfigCommands::Commit) => {
                    commit_host_config_to_db(hostname)?;
                }
                Some(ConfigCommands::Backup) => {
                    backup_host_config_to_env(hostname)?;
                }
                Some(ConfigCommands::Delete { from_env }) => {
                    delete_host_config(hostname, *from_env)?;
                }
                Some(ConfigCommands::Ip { value }) => {
                    set_host_field(hostname, "ip", &value)?;
                }
                Some(ConfigCommands::Hostname { value }) => {
                    set_host_field(hostname, "hostname", &value)?;
                }
                // Tailscale command removed - use hostname instead
                Some(ConfigCommands::BackupPath { value }) => {
                    set_host_field(hostname, "backup_path", &value)?;
                }
                Some(ConfigCommands::SetBackup { hostname: _ }) => {
                    // This shouldn't happen when hostname is provided, but handle it
                    set_backup_location(Some(hostname))?;
                }
                Some(ConfigCommands::Diff) => {
                    anyhow::bail!(
                        "Diff command is global only. Use 'halvor config diff' to see all differences"
                    );
                }
                _ => {
                    anyhow::bail!("Command not valid for hostname-specific operations");
                }
            }
            return Ok(());
        }
    }

    // Handle global config commands
    // If arg is a known command, use it; otherwise use the subcommand
    let cmd = if let Some(arg_str) = arg {
        // Map string to command
        match arg_str.to_lowercase().as_str() {
            "list" => ConfigCommands::List,
            "init" => ConfigCommands::Init,
            "env" => ConfigCommands::Env,
            "stable" => ConfigCommands::SetStable,
            "experimental" => ConfigCommands::SetExperimental,
            "commit" => ConfigCommands::Commit,
            "backup" => ConfigCommands::Backup,
            "diff" => ConfigCommands::Diff,
            _ => {
                // Use the subcommand if provided, otherwise default to Show
                command.cloned().unwrap_or(ConfigCommands::List)
            }
        }
    } else {
        // Use the subcommand if provided, otherwise default to Show
        command.cloned().unwrap_or(ConfigCommands::List)
    };

    match cmd {
        ConfigCommands::List => {
            if db {
                show_db_config(verbose)?;
            } else {
                show_current_config(verbose)?;
            }
        }
        ConfigCommands::Commit => {
            commit_all_to_db()?;
        }
        ConfigCommands::Backup => {
            backup_all_to_env_with_backup()?;
        }
        ConfigCommands::Init => {
            config_manager::init_config_interactive()?;
        }
        ConfigCommands::SetEnv { path } => {
            set_env_path(path.as_str())?;
        }
        ConfigCommands::SetStable => {
            config_manager::set_release_channel(config_manager::ReleaseChannel::Stable)?;
        }
        ConfigCommands::SetExperimental => {
            config_manager::set_release_channel(config_manager::ReleaseChannel::Experimental)?;
        }
        ConfigCommands::Create { command } => {
            handle_create_config(command)?;
        }
        ConfigCommands::Env => {
            create_example_env_file()?;
        }
        ConfigCommands::SetBackup { hostname } => {
            set_backup_location(hostname.as_deref())?;
        }
        ConfigCommands::Delete { .. } => {
            anyhow::bail!(
                "Delete requires a hostname. Usage: halvor config <hostname> delete [--from-env]"
            );
        }
        ConfigCommands::Diff => {
            show_config_diff()?;
        }
        ConfigCommands::Kubeconfig { setup, hostname } => {
            handle_kubeconfig_command(setup, hostname.as_deref())?;
        }
        ConfigCommands::Regenerate { hostname, yes } => {
            handle_regenerate_command(hostname.as_deref(), yes)?;
        }
        ConfigCommands::Ip { .. }
        | ConfigCommands::Hostname { .. }
        | ConfigCommands::BackupPath { .. } => {
            anyhow::bail!(
                "This command requires a hostname. Usage: halvor config <hostname> <command>"
            );
        }
    }

    Ok(())
}

/// Handle db commands
pub fn handle_db_command(command: crate::commands::config::DbCommands) -> Result<()> {
    match command {
        crate::commands::config::DbCommands::Generate => {
            db::core::generator::generate_structs()?;
        }
        crate::commands::config::DbCommands::Backup { path } => {
            backup_database(path.as_deref())?;
        }
        crate::commands::config::DbCommands::Migrate { command } => {
            // Default to running all migrations if no subcommand provided
            match command {
                Some(cmd) => handle_migrate_command(cmd)?,
                None => db::migrate::migrate_all()?,
            }
        }
        crate::commands::config::DbCommands::Sync => {
            sync_db_from_env()?;
        }
        crate::commands::config::DbCommands::Restore => {
            restore_database()?;
        }
    }
    Ok(())
}

/// Handle migrate commands
pub fn handle_migrate_command(command: crate::commands::config::MigrateCommands) -> Result<()> {
    match command {
        crate::commands::config::MigrateCommands::Up => {
            db::migrate::migrate_up()?;
        }
        crate::commands::config::MigrateCommands::Down => {
            db::migrate::migrate_down()?;
        }
        crate::commands::config::MigrateCommands::List => {
            db::migrate::migrate_list()?;
        }
        crate::commands::config::MigrateCommands::Generate { description }
        | crate::commands::config::MigrateCommands::GenerateShort { description } => {
            db::migrate::generate_migration(description)?;
        }
    }
    Ok(())
}

/// Sync environment file to database (load env values into DB, delete DB values not in env)
pub fn sync_db_from_env() -> Result<()> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Configuration Sync");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Configuration is only in .env (loaded from 1Password via .envrc)
    println!("✓ Configuration is stored in .env file (loaded from 1Password via .envrc)");
    println!("  No database sync needed - all configuration comes from .env");
    println!();

    Ok(())
}

/// Restore database from backup
pub fn restore_database() -> Result<()> {
    use glob::glob;
    use std::fs;
    use std::io::{self, Write};
    use std::path::PathBuf;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Restore Database from Backup");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Find all backup files
    let current_dir = std::env::current_dir()?;
    let backup_pattern = current_dir.join("halvor_backup_*.db");

    let mut backups: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = glob(backup_pattern.to_str().unwrap()) {
        for entry in entries.flatten() {
            backups.push(entry);
        }
    }

    // Also check halvor directory
    if let Ok(halvor_dir) = find_halvor_dir() {
        let backup_pattern = halvor_dir.join("halvor_backup_*.db");
        if let Ok(entries) = glob(backup_pattern.to_str().unwrap()) {
            for entry in entries.flatten() {
                if !backups.contains(&entry) {
                    backups.push(entry);
                }
            }
        }
    }

    if backups.is_empty() {
        anyhow::bail!("No backup files found. Look for files matching 'halvor_backup_*.db'");
    }

    // Sort by modification time (newest first)
    backups.sort_by(|a, b| {
        let a_time = fs::metadata(a)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let b_time = fs::metadata(b)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        b_time.cmp(&a_time)
    });

    println!("Available backups:");
    for (i, backup) in backups.iter().enumerate() {
        if let Ok(metadata) = fs::metadata(backup) {
            if let Ok(modified) = metadata.modified() {
                let datetime: chrono::DateTime<chrono::Utc> = modified.into();
                println!(
                    "  [{}] {} ({})",
                    i + 1,
                    backup.display(),
                    datetime.format("%Y-%m-%d %H:%M:%S")
                );
            } else {
                println!("  [{}] {}", i + 1, backup.display());
            }
        } else {
            println!("  [{}] {}", i + 1, backup.display());
        }
    }
    println!();

    print!("Select backup to restore [1]: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let selection = input.trim();

    let idx: usize = if selection.is_empty() {
        0
    } else {
        selection.parse().with_context(|| "Invalid selection")?
    };

    if idx < 1 || idx > backups.len() {
        anyhow::bail!("Invalid selection");
    }

    let backup_path = &backups[idx - 1];
    let db_path = db::get_db_path()?;

    // Backup current database before restore
    if db_path.exists() {
        use chrono::Utc;
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let current_backup = db_path
            .parent()
            .unwrap()
            .join(format!("halvor_pre_restore_{}.db", timestamp));
        fs::copy(&db_path, &current_backup).with_context(|| {
            format!(
                "Failed to backup current database to {}",
                current_backup.display()
            )
        })?;
        println!(
            "✓ Backed up current database to {}",
            current_backup.display()
        );
    }

    // Restore from backup
    fs::copy(backup_path, &db_path).with_context(|| {
        format!(
            "Failed to restore database from {} to {}",
            backup_path.display(),
            db_path.display()
        )
    })?;

    println!("✓ Database restored from {}", backup_path.display());
    println!();

    Ok(())
}

/// Handle kubeconfig command - print or setup kubectl context
fn handle_kubeconfig_command(setup: bool, hostname: Option<&str>) -> Result<()> {
    use crate::services::k3s::kubeconfig;
    use crate::utils::exec::local;

    let halvor_dir = find_halvor_dir()?;
    let config = load_env_config(&halvor_dir)?;

    // Determine primary hostname - use provided, or use first host in config
    let primary_hostname = if let Some(h) = hostname {
        h.to_string()
    } else {
        // Use first host in config
        config.hosts.keys()
            .next()
            .map(|name| name.clone())
            .ok_or_else(|| anyhow::anyhow!(
                "No hosts found in configuration. Please specify hostname with -H or add a host to your config"
            ))?
    };

    if setup {
        // Setup local kubectl context
        println!("Setting up kubectl context 'halvor'...");
        println!();

        // Check if kubectl exists
        if !local::check_command_exists("kubectl") {
            println!("  ✗ kubectl not found. Install kubectl first:");
            println!("     macOS: brew install kubectl");
            println!("     Linux: See https://kubernetes.io/docs/tasks/tools/");
            return Ok(());
        }

        // Fetch kubeconfig
        let mut kubeconfig_content = kubeconfig::fetch_kubeconfig_content(&primary_hostname, &config)?;

        // Simple string replacement to rename context and cluster to 'halvor'
        // This is more reliable than kubectl manipulation
        kubeconfig_content = kubeconfig_content.replace("name: default", "name: halvor");
        kubeconfig_content = kubeconfig_content.replace("cluster: default", "cluster: halvor");
        kubeconfig_content = kubeconfig_content.replace("user: default", "user: halvor");
        kubeconfig_content = kubeconfig_content.replace("current-context: default", "current-context: halvor");

        // Create temp file for kubeconfig
        let home = std::env::var("HOME").context("HOME environment variable not set")?;
        let kube_dir = format!("{}/.kube", home);
        std::fs::create_dir_all(&kube_dir).context("Failed to create ~/.kube directory")?;

        let main_config = format!("{}/.kube/config", home);
        let temp_config = format!("{}/.kube/halvor-temp.yaml", home);

        // Write processed kubeconfig to temp file
        std::fs::write(&temp_config, &kubeconfig_content)
            .context("Failed to write temporary kubeconfig")?;

        println!("  Configuring context 'halvor'...");

        // Backup existing config if it exists
        if std::path::Path::new(&main_config).exists() {
            let backup = format!("{}.backup-{}", main_config, chrono::Utc::now().timestamp());
            std::fs::copy(&main_config, &backup)
                .context("Failed to backup existing kubeconfig")?;
            println!("  Created backup at {}", backup);
        }

        // Merge configs - use kubectl config view with KUBECONFIG env var
        let merge_cmd = format!(
            "export KUBECONFIG='{}:{}' && kubectl config view --flatten > /tmp/kube-merged.yaml && mv /tmp/kube-merged.yaml '{}'",
            main_config, temp_config, main_config
        );

        let merge_result = local::execute_shell(&merge_cmd);
        if merge_result.is_err() {
            // If merge fails and no existing config, just copy temp config
            if !std::path::Path::new(&main_config).exists() {
                std::fs::copy(&temp_config, &main_config)?;
            }
        }

        // Set halvor as current context
        local::execute_shell("kubectl config use-context halvor")?;

        // Clean up temp file
        if std::path::Path::new(&temp_config).exists() {
            std::fs::remove_file(&temp_config)?;
        }

        println!("  ✓ Kubeconfig set up at {}", main_config);
        println!("  ✓ Context 'halvor' added and set as current");

        // Test connection
        println!();
        println!("Testing connection...");
        let test_result = local::execute_shell("kubectl cluster-info");
        if let Ok(output) = test_result {
            if output.status.success() {
                let info = String::from_utf8_lossy(&output.stdout);
                println!("{}", info);
                println!("✓ Successfully connected to halvor cluster");
            } else {
                println!("⚠ kubectl configured but connection test failed");
                println!("  Make sure Tailscale is running and connected");
            }
        }
    } else {
        // Just print kubeconfig for 1Password
        // Fetch kubeconfig first (this prints status messages)
        let kubeconfig_content = kubeconfig::fetch_kubeconfig_content(&primary_hostname, &config)?;

        // Fetch K3s join token
        println!();
        println!("  Fetching K3s join token...");
        let join_token = match crate::services::k3s::get_cluster_join_info(&primary_hostname, &config) {
            Ok((_, token)) => {
                println!("  ✓ Join token fetched");
                Some(token)
            }
            Err(e) => {
                println!("  ⚠ Failed to fetch join token: {}", e);
                println!("  You can add K3S_TOKEN to 1Password manually later if needed");
                None
            }
        };

        // Now print the formatted output with clear markers
        println!();
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("K3s Configuration for 1Password");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!();
        println!("📋 ADD THESE FIELDS TO YOUR 1PASSWORD VAULT");
        println!();

        // Print KUBE_CONFIG
        println!("Field 1: KUBE_CONFIG");
        println!("  Type: Concealed (password/secret)");
        println!("  Value: (copy content between markers below)");
        println!();
        println!("╔════════════════════ START KUBE_CONFIG ════════════════════╗");
        println!("{}", kubeconfig_content);
        println!("╚════════════════════ END KUBE_CONFIG ═════════════════════╝");
        println!();

        // Print K3S_TOKEN if available
        if let Some(token) = join_token {
            println!("Field 2: K3S_TOKEN");
            println!("  Type: Concealed (password/secret)");
            println!("  Value: (copy content between markers below)");
            println!();
            println!("╔════════════════════ START K3S_TOKEN ══════════════════════╗");
            println!("{}", token);
            println!("╚════════════════════ END K3S_TOKEN ════════════════════════╝");
            println!();
        }

        println!("After adding to 1Password:");
        println!("  1. Make sure your .envrc or environment loads these from 1Password");
        println!("  2. Run: halvor config kubeconfig --setup");
        println!("     (This will configure kubectl to use the 'halvor' context)");
        println!();
        println!("Usage:");
        println!("  • KUBE_CONFIG: Used for kubectl access and auto-detecting server in join commands");
        println!("  • K3S_TOKEN: Used for joining new nodes to the cluster");
        println!();

        // Prompt user to automatically update 1Password
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!();
        print!("Would you like to automatically update these values in 1Password? [y/N]: ");
        std::io::stdout().flush().context("Failed to flush stdout")?;

        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .context("Failed to read user input")?;

        if input.trim().eq_ignore_ascii_case("y") || input.trim().eq_ignore_ascii_case("yes") {
            println!();
            println!("Updating 1Password vault...");

            // Check if op CLI is available
            if !local::check_command_exists("op") {
                println!("⚠ 1Password CLI (op) not found. Please install it:");
                println!("   macOS: brew install --cask 1password-cli");
                println!("   Linux: See https://developer.1password.com/docs/cli/get-started/");
                return Ok(());
            }

            // Check if signed in
            let whoami = local::execute_shell("op whoami 2>&1");
            if whoami.is_err() || !whoami.as_ref().unwrap().status.success() {
                println!("⚠ Not signed in to 1Password CLI. Please sign in first:");
                println!("   eval $(op signin)");
                return Ok(());
            }

            // Get vault and item name from environment or use defaults
            let vault_name = std::env::var("OP_VAULT").unwrap_or_else(|_| "automations".to_string());
            let item_name = std::env::var("OP_ITEM").unwrap_or_else(|_| "halvor".to_string());

            // Update KUBE_CONFIG field
            println!("  Updating KUBE_CONFIG field...");
            let kube_config_escaped = kubeconfig_content.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n");
            let update_kube_cmd = format!(
                "op item edit '{}' --vault '{}' 'KUBE_CONFIG[concealed]={}'",
                item_name, vault_name, kube_config_escaped
            );

            match local::execute_shell(&update_kube_cmd) {
                Ok(output) => {
                    if output.status.success() {
                        println!("  ✓ KUBE_CONFIG updated");
                    } else {
                        let error = String::from_utf8_lossy(&output.stderr);
                        println!("  ⚠ Failed to update KUBE_CONFIG: {}", error);
                    }
                }
                Err(e) => {
                    println!("  ⚠ Failed to update KUBE_CONFIG: {}", e);
                }
            }

            // Update K3S_TOKEN field if available
            if let Some(ref token) = join_token {
                println!("  Updating K3S_TOKEN field...");
                let token_escaped = token.replace("\\", "\\\\").replace("\"", "\\\"");
                let update_token_cmd = format!(
                    "op item edit '{}' --vault '{}' 'K3S_TOKEN[concealed]={}'",
                    item_name, vault_name, token_escaped
                );

                match local::execute_shell(&update_token_cmd) {
                    Ok(output) => {
                        if output.status.success() {
                            println!("  ✓ K3S_TOKEN updated");
                        } else {
                            let error = String::from_utf8_lossy(&output.stderr);
                            println!("  ⚠ Failed to update K3S_TOKEN: {}", error);
                        }
                    }
                    Err(e) => {
                        println!("  ⚠ Failed to update K3S_TOKEN: {}", e);
                    }
                }
            }

            println!();
            println!("✓ 1Password vault updated!");
            println!();
            println!("Next steps:");
            println!("  1. Reload your environment: direnv allow (or restart your shell)");
            println!("  2. Verify: echo $KUBE_CONFIG | head -5");
            println!("  3. Setup kubectl: halvor config kubeconfig --setup");
        } else {
            println!();
            println!("Skipped automatic update. You can manually copy the values above to 1Password.");
        }
        println!();
    }

    Ok(())
}

fn handle_regenerate_command(hostname: Option<&str>, yes: bool) -> Result<()> {
    use crate::services::k3s;

    let halvor_dir = find_halvor_dir()?;
    let config = load_env_config(&halvor_dir)?;

    // Default to localhost if not provided
    let target_host = hostname.unwrap_or("localhost");

    // Regenerate certificates with Tailscale integration
    k3s::regenerate_certificates(target_host, yes, &config)?;

    Ok(())
}
