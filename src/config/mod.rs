use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

pub mod config_manager;
pub mod env_file;
pub mod service;

#[derive(Clone, Serialize, Deserialize)]
pub struct HostConfig {
    pub ip: Option<String>,
    pub hostname: Option<String>, // Hostname (typically Tailscale hostname)
    pub backup_path: Option<String>,
    pub sudo_password: Option<String>, // Sudo password from 1Password (HOST_<name>_SUDO_PASS)
    pub sudo_user: Option<String>,     // Sudo user from 1Password (HOST_<name>_SUDO_USER)
}

pub struct SmbServerConfig {
    pub host: String,
    pub shares: Vec<String>, // Multiple shares per server
    pub username: Option<String>,
    pub password: Option<String>,
    pub options: Option<String>,
}

pub struct EnvConfig {
    pub _tailnet_base: String,
    pub hosts: HashMap<String, HostConfig>,
    pub smb_servers: HashMap<String, SmbServerConfig>,
}

pub fn find_halvor_dir() -> Result<PathBuf> {
    use crate::config::config_manager;

    // Check for environment variable override (support both new and legacy names)
    if let Ok(dir) = env::var("HALVOR_DIR") {
        return Ok(PathBuf::from(dir));
    }
    if let Ok(dir) = env::var("HOMELAB_DIR") {
        return Ok(PathBuf::from(dir));
    }

    // Check if env file path is configured in hal config
    if let Some(env_path) = config_manager::get_env_file_path() {
        if let Some(parent) = env_path.parent() {
            return Ok(parent.to_path_buf());
        }
    }

    // Try to find .env file in current directory or parent directories
    let mut current = env::current_dir()?;
    loop {
        let env_file = current.join(".env");
        if env_file.exists() {
            return Ok(current);
        }
        if !current.pop() {
            break;
        }
    }

    // Fallback: use current directory
    Ok(env::current_dir()?)
}

/// Alias for backward compatibility
#[inline]
pub fn find_homelab_dir() -> Result<PathBuf> {
    find_halvor_dir()
}

pub fn get_env_file_path() -> Result<PathBuf> {
    use crate::config::config_manager;

    // Check for environment variable override (support both new and legacy names)
    if let Ok(path) = env::var("HALVOR_ENV_FILE") {
        return Ok(PathBuf::from(path));
    }
    if let Ok(path) = env::var("HOMELAB_ENV_FILE") {
        return Ok(PathBuf::from(path));
    }

    // Check if env file path is configured in hal config
    if let Some(env_path) = config_manager::get_env_file_path() {
        return Ok(env_path);
    }

    // Fallback: try to find .env in halvor directory
    let halvor_dir = find_halvor_dir()?;
    Ok(halvor_dir.join(".env"))
}

pub fn load_env_config(_halvor_dir: &Path) -> Result<EnvConfig> {
    let env_file = get_env_file_path()?;

    // Load .env file if it exists, but don't require it
    // Environment variables may already be set via direnv/.envrc
    if env_file.exists() {
        dotenv::from_path(&env_file)
            .with_context(|| format!("Failed to load .env file from {}", env_file.display()))?;
    }

    let tailnet_base = env::var("TAILNET_BASE").unwrap_or_else(|_| "ts.net".to_string());

    // Parse host configurations
    let mut hosts = HashMap::new();
    let mut smb_servers = HashMap::new();
    let env_vars: Vec<(String, String)> = env::vars().collect();

    for (key, value) in env_vars {
        if let Some(hostname) = key.strip_prefix("HOST_") {
            // Check for _TAILSCALE_IP first (before _IP) to avoid parsing it as a separate host
            if let Some(rest) = hostname.strip_suffix("_TAILSCALE_IP") {
                // Tailscale IP - use as primary IP if no regular IP is set
                let hostname_lower = rest.to_lowercase();
                let config = hosts.entry(hostname_lower).or_insert_with(|| HostConfig {
                    ip: None,
                    hostname: None,
                    backup_path: None,
                    sudo_password: None,
                    sudo_user: None,
                });
                // Only set IP if not already set by HOST_<name>_IP
                if config.ip.is_none() {
                    config.ip = Some(value);
                }
            } else if let Some(rest) = hostname.strip_suffix("_IP") {
                let hostname_lower = rest.to_lowercase();
                let config = hosts.entry(hostname_lower).or_insert_with(|| HostConfig {
                    ip: None,
                    hostname: None,
                    backup_path: None,
                    sudo_password: None,
                    sudo_user: None,
                });
                config.ip = Some(value);
            } else if let Some(rest) = hostname.strip_suffix("_HOSTNAME") {
                let hostname_lower = rest.to_lowercase();
                let config = hosts.entry(hostname_lower).or_insert_with(|| HostConfig {
                    ip: None,
                    hostname: None,
                    backup_path: None,
                    sudo_password: None,
                    sudo_user: None,
                });
                config.hostname = Some(value);
            } else if let Some(rest) = hostname.strip_suffix("_BACKUP_PATH") {
                let hostname_lower = rest.to_lowercase();
                let config = hosts.entry(hostname_lower).or_insert_with(|| HostConfig {
                    ip: None,
                    hostname: None,
                    backup_path: None,
                    sudo_password: None,
                    sudo_user: None,
                });
                config.backup_path = Some(value);
            } else if let Some(rest) = hostname.strip_suffix("_SUDO_PASS") {
                let hostname_lower = rest.to_lowercase();
                let config = hosts.entry(hostname_lower).or_insert_with(|| HostConfig {
                    ip: None,
                    hostname: None,
                    backup_path: None,
                    sudo_password: None,
                    sudo_user: None,
                });
                config.sudo_password = Some(value);
            } else if let Some(rest) = hostname.strip_suffix("_SUDO_USER") {
                let hostname_lower = rest.to_lowercase();
                let config = hosts.entry(hostname_lower).or_insert_with(|| HostConfig {
                    ip: None,
                    hostname: None,
                    backup_path: None,
                    sudo_password: None,
                    sudo_user: None,
                });
                config.sudo_user = Some(value);
            }
        } else if let Some(server_name) = key.strip_prefix("SMB_") {
            // Parse SMB server configuration
            // Format: SMB_<SERVERNAME>_<PROPERTY>
            // Properties: HOST, SHARES (comma-separated), USERNAME, PASSWORD, OPTIONS
            let parts: Vec<&str> = server_name.split('_').collect();
            if parts.len() >= 2 {
                let server_name_lower = parts[0].to_lowercase();
                let property = parts[1..].join("_");

                let server_config =
                    smb_servers
                        .entry(server_name_lower)
                        .or_insert_with(|| SmbServerConfig {
                            host: String::new(),
                            shares: Vec::new(),
                            username: None,
                            password: None,
                            options: None,
                        });

                match property.as_str() {
                    "HOST" => server_config.host = value,
                    "SHARES" => {
                        // Parse comma-separated shares
                        server_config.shares = value
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                    "SHARE" => {
                        // Legacy support: single share (adds to shares vec)
                        let share = value.trim().to_string();
                        if !share.is_empty() && !server_config.shares.contains(&share) {
                            server_config.shares.push(share);
                        }
                    }
                    "USERNAME" => server_config.username = Some(value),
                    "PASSWORD" => server_config.password = Some(value),
                    "OPTIONS" => server_config.options = Some(value),
                    _ => {}
                }
            }
        }
    }

    // Validate SMB server configurations
    for (name, config) in &smb_servers {
        if config.host.is_empty() {
            anyhow::bail!(
                "SMB server '{}' is missing required configuration (HOST)",
                name
            );
        }
        if config.shares.is_empty() {
            anyhow::bail!(
                "SMB server '{}' is missing required configuration (SHARES or SHARE)",
                name
            );
        }
    }

    Ok(EnvConfig {
        _tailnet_base: tailnet_base,
        hosts,
        smb_servers,
    })
}

pub fn get_default_username() -> String {
    env::var("USER")
        .or_else(|_| env::var("USERNAME"))
        .unwrap_or_else(|_| "root".to_string())
}

pub fn get_os() -> &'static str {
    env::consts::OS
}

pub fn get_arch() -> &'static str {
    env::consts::ARCH
}

pub fn get_npm_url() -> Option<String> {
    env::var("NPM_URL").ok()
}

pub fn get_npm_username() -> Option<String> {
    env::var("NPM_USERNAME").ok()
}

pub fn get_npm_password() -> Option<String> {
    env::var("NPM_PASSWORD").ok()
}

/// Helper function to load config - used by commands and services
/// Only uses .env file (loaded from 1Password via .envrc)
pub fn load_config() -> Result<EnvConfig> {
    #[cfg(debug_assertions)]
    println!("[DEBUG] Loading configuration...");

    let halvor_dir = find_halvor_dir()?;
    let env_config = load_env_config(&halvor_dir)?;

    #[cfg(debug_assertions)]
    println!(
        "[DEBUG] Loaded {} hosts from .env file",
        env_config.hosts.len()
    );

    // Only use .env config (loaded from 1Password via .envrc)

    #[cfg(debug_assertions)]
    println!("[DEBUG] Final config has {} hosts", env_config.hosts.len());

    Ok(env_config)
}
