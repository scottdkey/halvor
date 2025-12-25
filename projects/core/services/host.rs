// Host service - all host-related business logic
use crate::config::{HostConfig, find_halvor_dir, load_env_config};
use crate::db;
use crate::utils::exec::Executor;
use anyhow::{Context, Result};

/// Get host configuration from .env file (loaded from 1Password)
/// This is the main entry point for getting host configuration
pub fn get_host_config(hostname: &str) -> Result<Option<HostConfig>> {
    let halvor_dir = find_halvor_dir()?;
    let config = load_env_config(&halvor_dir)?;
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

/// List all known hosts from .env file
pub fn list_hosts() -> Result<Vec<String>> {
    let halvor_dir = find_halvor_dir()?;
    let config = load_env_config(&halvor_dir)?;
    let mut hosts: Vec<String> = config.hosts.keys().cloned().collect();
    hosts.sort();
    Ok(hosts)
}

/// Store host configuration to .env file
pub fn store_host_config(hostname: &str, config: &HostConfig) -> Result<()> {
    let halvor_dir = find_halvor_dir()?;
    let env_path = halvor_dir.join(".env");
    crate::config::env_file::write_host_to_env_file(&env_path, hostname, config)
}

/// Delete host configuration from .env file
pub fn delete_host_config(hostname: &str) -> Result<()> {
    let halvor_dir = find_halvor_dir()?;
    let env_path = halvor_dir.join(".env");
    crate::config::env_file::remove_host_from_env_file(&env_path, hostname)
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
    let halvor_dir = find_halvor_dir()?;
    let config = load_env_config(&halvor_dir)?;
    Executor::new(hostname, &config)
}

/// List all hosts with their information
pub fn list_hosts_display(verbose: bool) -> Result<()> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Available Servers");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Try to load from env file
    let halvor_dir = crate::config::find_halvor_dir();
    let (env_hosts, tailnet_base) = if let Ok(dir) = &halvor_dir {
        match crate::config::load_env_config(dir) {
            Ok(cfg) => {
                #[cfg(debug_assertions)]
                println!(
                    "[DEBUG] Loaded {} hosts from .env file in list_hosts_display",
                    cfg.hosts.len()
                );
                (Some(cfg.hosts), cfg._tailnet_base)
            }
            Err(e) => {
                eprintln!("Failed to load .env config: {}", e);
                (None, "ts.net".to_string())
            }
        }
    } else {
        (None, "ts.net".to_string())
    };

    // Only use .env config (loaded from 1Password)
    let mut all_hosts = std::collections::HashMap::new();

    if let Some(hosts) = env_hosts {
        for (name, config) in hosts {
            all_hosts.insert(name, config);
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
            let config = all_hosts.get(*hostname).unwrap();
            println!("Hostname: {}", hostname);
            if let Some(ref ip) = config.ip {
                println!("  IP Address: {}", ip);
            }
            if let Some(ref hostname_val) = config.hostname {
                println!("  Hostname: {}.{}", hostname_val, tailnet_base);
            }
            if let Some(ref backup_path) = config.backup_path {
                println!("  Backup Path: {}", backup_path);
            }
            println!();
        }
    } else {
        println!("Servers:");
        for hostname in &hostnames {
            let config = all_hosts.get(*hostname).unwrap();
            let mut info = vec![];
            if let Some(ref ip) = config.ip {
                info.push(format!("IP: {}", ip));
            }
            if let Some(ref hostname_val) = config.hostname {
                info.push(format!("Host: {}", hostname_val));
            }
            if info.is_empty() {
                println!("  {}", hostname);
            } else {
                println!("  {} ({})", hostname, info.join(", "));
            }
        }
        println!();
        println!("Use 'halvor list --verbose' for detailed information.");
    }

    Ok(())
}
