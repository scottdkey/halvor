use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const CONFIG_DIR_NAME: &str = "halvor";
const CONFIG_FILE_NAME: &str = "config.toml";

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseChannel {
    Stable,
    Experimental,
}

impl Default for ReleaseChannel {
    fn default() -> Self {
        ReleaseChannel::Stable
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HalConfig {
    pub env_file_path: Option<PathBuf>,
    #[serde(default)]
    pub release_channel: ReleaseChannel,
}

impl Default for HalConfig {
    fn default() -> Self {
        Self {
            env_file_path: None,
            release_channel: ReleaseChannel::Stable,
        }
    }
}

pub fn get_config_dir() -> Result<PathBuf> {
    let home = get_home_dir()?;
    let config_dir = home.join(".config").join(CONFIG_DIR_NAME);

    // Create config directory if it doesn't exist
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).with_context(|| {
            format!(
                "Failed to create config directory: {}",
                config_dir.display()
            )
        })?;
    }

    Ok(config_dir)
}

pub fn get_config_file_path() -> Result<PathBuf> {
    let config_dir = get_config_dir()?;
    Ok(config_dir.join(CONFIG_FILE_NAME))
}

pub fn get_home_dir() -> Result<PathBuf> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE")) // Windows fallback
        .map(PathBuf::from)
        .with_context(|| "Could not determine home directory")
}

pub fn load_config() -> Result<HalConfig> {
    let config_path = get_config_file_path()?;

    if !config_path.exists() {
        return Ok(HalConfig::default());
    }

    let content = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

    let config: HalConfig = toml::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", config_path.display()))?;

    Ok(config)
}

pub fn save_config(config: &HalConfig) -> Result<()> {
    let config_path = get_config_file_path()?;
    let content = toml::to_string_pretty(config).context("Failed to serialize config")?;

    fs::write(&config_path, content)
        .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;

    Ok(())
}

pub fn set_env_file_path(env_path: &Path) -> Result<()> {
    let mut config = load_config().unwrap_or_default();
    config.env_file_path = Some(env_path.to_path_buf());
    save_config(&config)?;

    println!("✓ Environment file path configured: {}", env_path.display());
    Ok(())
}

pub fn get_env_file_path() -> Option<PathBuf> {
    load_config().ok()?.env_file_path
}

pub fn prompt_for_env_file() -> Result<PathBuf> {
    print!("Enter path to your .env file: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let path_str = input.trim();

    if path_str.is_empty() {
        anyhow::bail!("Path cannot be empty");
    }

    let path = PathBuf::from(path_str);

    // Expand ~ to home directory
    let path = if path_str.starts_with("~") {
        let home = get_home_dir()?;
        home.join(path_str.strip_prefix("~/").unwrap_or(""))
    } else {
        path
    };

    // Resolve to absolute path
    let path = if path.is_relative() {
        std::env::current_dir()?.join(path)
    } else {
        path
    };

    // Normalize the path
    let path = path
        .canonicalize()
        .with_context(|| format!("Failed to resolve path: {}", path_str))?;

    // Verify the file exists
    if !path.exists() {
        anyhow::bail!("File does not exist: {}", path.display());
    }

    if !path.is_file() {
        anyhow::bail!("Path is not a file: {}", path.display());
    }

    Ok(path)
}

pub fn init_config_interactive() -> Result<()> {
    println!("HAL Configuration Setup");
    println!("======================");
    println!();

    let config = load_config()?;

    // Show current configuration summary
    show_config_summary()?;

    if let Some(ref env_path) = config.env_file_path {
        println!();
        println!("Current environment file: {}", env_path.display());
        print!("Change it? [y/N]: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if input.trim().to_lowercase() != "y" {
            println!("Configuration unchanged.");
            return Ok(());
        }
    }

    println!();
    println!("Please provide the path to your .env file.");
    println!("This file contains your host configurations and credentials.");
    println!();

    // Try to find default .env file location
    let default_env = get_default_env_path()?;
    if default_env.exists() {
        println!("Found .env file at: {}", default_env.display());
        print!("Use this location? [Y/n]: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if input.trim().is_empty()
            || input.trim().to_lowercase() == "y"
            || input.trim().to_lowercase() == "yes"
        {
            set_env_file_path(&default_env)?;
            println!();
            println!("✓ Configuration saved!");
            println!("  Config location: {}", get_config_file_path()?.display());
            println!("  Environment file: {}", default_env.display());
            return Ok(());
        }
    }

    let env_path = prompt_for_env_file()?;
    set_env_file_path(&env_path)?;

    println!();
    println!("✓ Configuration saved!");
    println!("  Config location: {}", get_config_file_path()?.display());
    println!("  Environment file: {}", env_path.display());

    Ok(())
}

/// Get default .env file path (in user's home directory)
fn get_default_env_path() -> Result<PathBuf> {
    let home = get_home_dir()?;
    Ok(home.join(".env"))
}

/// Show configuration summary from .env (loaded from 1Password)
fn show_config_summary() -> Result<()> {
    use crate::config;

    println!("Configuration Summary");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Get servers from env file
    let halvor_dir = config::find_halvor_dir();
    let env_hosts = if let Ok(dir) = &halvor_dir {
        match config::load_env_config(dir) {
            Ok(cfg) => Some(cfg.hosts),
            Err(_) => None,
        }
    } else {
        None
    };

    // Show .env file servers (loaded from 1Password)
    if let Some(hosts) = &env_hosts {
        if !hosts.is_empty() {
            println!("Servers in .env file (from 1Password):");
            let mut hostnames: Vec<_> = hosts.keys().collect();
            hostnames.sort();
            for hostname in hostnames {
                let host_config = &hosts[hostname];
                let mut info = vec![];
                if host_config.ip.is_some() {
                    info.push("IP");
                }
                if host_config.hostname.is_some() {
                    info.push("Hostname");
                }
                if host_config.backup_path.is_some() {
                    info.push("Backup");
                }
                println!("  • {} ({})", hostname, info.join(", "));
            }
        } else {
            println!("No servers found in .env file");
        }
    } else {
        println!("No .env file found or could not be loaded");
    }

    println!();

    Ok(())
}

pub fn set_release_channel(channel: ReleaseChannel) -> Result<()> {
    let mut config = load_config().unwrap_or_default();
    config.release_channel = channel;
    save_config(&config)?;

    let channel_name = match channel {
        ReleaseChannel::Stable => "stable",
        ReleaseChannel::Experimental => "experimental",
    };
    println!("✓ Release channel set to: {}", channel_name);
    Ok(())
}

pub fn get_release_channel() -> ReleaseChannel {
    load_config().unwrap_or_default().release_channel
}
