use crate::config::HostConfig;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

/// Write host configuration to .env file
pub fn write_host_to_env_file(
    env_path: &PathBuf,
    hostname: &str,
    config: &HostConfig,
) -> Result<()> {
    // Read existing .env file
    let content = if env_path.exists() {
        fs::read_to_string(env_path)
            .with_context(|| format!("Failed to read .env file: {}", env_path.display()))?
    } else {
        String::new()
    };

    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    let hostname_upper = hostname.to_uppercase();

    // Remove existing entries for this host
    lines.retain(|line| {
        let trimmed = line.trim();
        !trimmed.starts_with('#') && !trimmed.starts_with(&format!("HOST_{}_", hostname_upper))
    });

    // Add new entries
    if let Some(ref ip) = config.ip {
        lines.push(format!("HOST_{}_IP={}", hostname_upper, ip));
    }
    if let Some(ref hostname_val) = config.hostname {
        lines.push(format!("HOST_{}_HOSTNAME={}", hostname_upper, hostname_val));
    }
    if let Some(ref backup_path) = config.backup_path {
        lines.push(format!(
            "HOST_{}_BACKUP_PATH={}",
            hostname_upper, backup_path
        ));
    }

    // Write back to file
    fs::write(env_path, lines.join("\n") + "\n")
        .with_context(|| format!("Failed to write .env file: {}", env_path.display()))?;

    Ok(())
}

/// Remove host configuration from .env file
pub fn remove_host_from_env_file(env_path: &PathBuf, hostname: &str) -> Result<()> {
    if !env_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(env_path)
        .with_context(|| format!("Failed to read .env file: {}", env_path.display()))?;

    let hostname_upper = hostname.to_uppercase();
    let lines: Vec<String> = content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed.starts_with('#') || !trimmed.starts_with(&format!("HOST_{}_", hostname_upper))
        })
        .map(|s| s.to_string())
        .collect();

    fs::write(env_path, lines.join("\n") + "\n")
        .with_context(|| format!("Failed to write .env file: {}", env_path.display()))?;

    Ok(())
}
