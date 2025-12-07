use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const CONFIG_DIR_NAME: &str = "hal";
const CONFIG_FILE_NAME: &str = "config.toml";

#[derive(Debug, Serialize, Deserialize)]
pub struct HalConfig {
    pub env_file_path: Option<PathBuf>,
}

impl Default for HalConfig {
    fn default() -> Self {
        Self {
            env_file_path: None,
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

    if let Some(ref env_path) = config.env_file_path {
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

    let env_path = prompt_for_env_file()?;
    set_env_file_path(&env_path)?;

    println!();
    println!("✓ Configuration saved!");
    println!("  Config location: {}", get_config_file_path()?.display());
    println!("  Environment file: {}", env_path.display());

    Ok(())
}
