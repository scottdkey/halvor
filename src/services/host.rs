// Host service - all host-related business logic
use crate::config::{HostConfig, find_homelab_dir, load_env_config};
use crate::db;
use crate::utils::exec::Executor;
use anyhow::{Context, Result};

/// Get host configuration from config or database
/// This is the main entry point for getting host configuration
pub fn get_host_config(hostname: &str) -> Result<Option<HostConfig>> {
    // Try database first if configured to use DB
    if let Ok(Some(config)) = db::get_host_config(hostname) {
        return Ok(Some(config));
    }

    // Fallback to .env config
    let homelab_dir = find_homelab_dir()?;
    let config = load_env_config(&homelab_dir)?;
    Ok(config.hosts.get(hostname).cloned())
}

/// Get host configuration with error message if not found
pub fn get_host_config_or_error(hostname: &str) -> Result<HostConfig> {
    get_host_config(hostname)?.with_context(|| {
        format!(
            "Host '{}' not found\n\nAdd configuration:\n  halvor config create ssh {}",
            hostname, hostname
        )
    })
}

/// List all known hosts
pub fn list_hosts() -> Result<Vec<String>> {
    // Try database first
    if let Ok(hosts) = db::list_hosts() {
        if !hosts.is_empty() {
            return Ok(hosts);
        }
    }

    // Fallback to .env config
    let homelab_dir = find_homelab_dir()?;
    let config = load_env_config(&homelab_dir)?;
    let mut hosts: Vec<String> = config.hosts.keys().cloned().collect();
    hosts.sort();
    Ok(hosts)
}

/// Store host configuration
pub fn store_host_config(hostname: &str, config: &HostConfig) -> Result<()> {
    db::store_host_config(hostname, config)
}

/// Delete host configuration
pub fn delete_host_config(hostname: &str) -> Result<()> {
    db::delete_host_config(hostname)
}

/// Store host provisioning information
pub fn store_host_info(
    hostname: &str,
    docker_version: Option<&str>,
    tailscale_installed: bool,
    portainer_installed: bool,
    metadata: Option<&str>,
) -> Result<()> {
    db::store_host_info(
        hostname,
        docker_version,
        tailscale_installed,
        portainer_installed,
        metadata,
    )
}

/// Get host provisioning information
pub fn get_host_info(
    hostname: &str,
) -> Result<Option<(Option<i64>, Option<String>, bool, bool, Option<String>)>> {
    db::get_host_info(hostname)
}

/// Create an executor for a host (local or remote)
pub fn create_executor(hostname: &str) -> Result<Executor> {
    let homelab_dir = find_homelab_dir()?;
    let config = load_env_config(&homelab_dir)?;
    Executor::new(hostname, &config)
}

/// List all hosts with their information
pub fn list_hosts_display(verbose: bool) -> Result<()> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Available Servers");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Try to load from env file
    let homelab_dir = crate::config::find_homelab_dir();
    let (env_hosts, tailnet_base) = if let Ok(dir) = &homelab_dir {
        match crate::config::load_env_config(dir) {
            Ok(cfg) => (Some(cfg.hosts), cfg._tailnet_base),
            Err(_) => (None, "ts.net".to_string()),
        }
    } else {
        (None, "ts.net".to_string())
    };

    // Try to load from database
    let db_hosts = db::list_hosts().ok();

    // Combine hosts from both sources
    let mut all_hosts = std::collections::HashMap::new();

    if let Some(hosts) = env_hosts {
        for (name, config) in hosts {
            all_hosts.insert(name, ("env", config));
        }
    }

    if let Some(hosts) = db_hosts {
        for name in hosts {
            // Get host config from DB if available
            if let Ok(Some(config)) = get_host_config(&name) {
                // If host exists in both, mark as "both"
                if let Some((source, _)) = all_hosts.get_mut(&name) {
                    *source = "both";
                } else {
                    all_hosts.insert(name, ("db", config));
                }
            } else if !all_hosts.contains_key(&name) {
                // Host exists in DB but no config - create empty config
                let empty_config = HostConfig {
                    ip: None,
                    hostname: None,
                    tailscale: None,
                    backup_path: None,
                };
                all_hosts.insert(name, ("db", empty_config));
            }
        }
    }

    if all_hosts.is_empty() {
        println!("No servers found.");
        println!();
        println!("To add servers:");
        println!("  halvor config create ssh <hostname>");
        return Ok(());
    }

    // Sort hostnames for consistent output
    let mut hostnames: Vec<_> = all_hosts.keys().collect();
    hostnames.sort();

    if verbose {
        for hostname in &hostnames {
            let (source, config) = all_hosts.get(*hostname).unwrap();
            println!("Hostname: {}", hostname);
            println!(
                "  Source: {}",
                match *source {
                    "env" => "Environment file (.env)",
                    "db" => "Database (SQLite)",
                    "both" => "Environment file & Database",
                    _ => "Unknown",
                }
            );
            if let Some(ref ip) = config.ip {
                println!("  IP Address: {}", ip);
            }
            if let Some(ref hostname) = config.hostname {
                println!("  Hostname: {}.{}", hostname, tailnet_base);
            }
            if let Some(ref tailscale) = config.tailscale {
                if config
                    .hostname
                    .as_ref()
                    .map(|h| h != tailscale)
                    .unwrap_or(true)
                {
                    println!(
                        "  Tailscale: {}.{} (different from hostname)",
                        tailscale, tailnet_base
                    );
                }
            }
            if let Some(ref backup_path) = config.backup_path {
                println!("  Backup Path: {}", backup_path);
            }
            // Get provisioning info from DB if available
            if let Ok(Some(info)) = get_host_info(hostname) {
                if let Some(ref docker_version) = info.1 {
                    println!("  Docker Version: {}", docker_version);
                }
                println!(
                    "  Tailscale Installed: {}",
                    if info.2 { "Yes" } else { "No" }
                );
                println!(
                    "  Portainer Installed: {}",
                    if info.3 { "Yes" } else { "No" }
                );
                if let Some(ref metadata) = info.4 {
                    println!("  Metadata: {}", metadata);
                }
            }
            println!();
        }
    } else {
        println!("Servers:");
        for hostname in &hostnames {
            let (source, config) = all_hosts.get(*hostname).unwrap();
            let mut info = vec![];
            if let Some(ref ip) = config.ip {
                info.push(format!("IP: {}", ip));
            }
            if let Some(ref hostname) = config.hostname {
                info.push(format!("Host: {}", hostname));
            }
            if let Some(ref tailscale) = config.tailscale {
                if config
                    .hostname
                    .as_ref()
                    .map(|h| h != tailscale)
                    .unwrap_or(true)
                {
                    info.push(format!("TS: {}", tailscale));
                }
            }
            let source_marker = match *source {
                "env" => "[env]",
                "db" => "[db]",
                "both" => "[env+db]",
                _ => "",
            };
            if info.is_empty() {
                println!("  {} {}", hostname, source_marker);
            } else {
                println!("  {} {} ({})", hostname, source_marker, info.join(", "));
            }
        }
        println!();
        println!("Use 'halvor list --verbose' for detailed information.");
    }

    Ok(())
}
