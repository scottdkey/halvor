use crate::config::EnvConfig;
use crate::db;
use crate::utils::{bytes_to_string, ssh::SshConnection};
use anyhow::{Context, Result};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use std::path::PathBuf;

/// Sync data to/from a remote halvor installation
pub fn sync_data(hostname: &str, pull: bool, config: &EnvConfig) -> Result<()> {
    // Get target host info
    let host_config = config
        .hosts
        .get(hostname)
        .with_context(|| format!("Host '{}' not found in configuration", hostname))?;

    let target_host = if let Some(ref tailscale) = host_config.tailscale {
        format!("{}.{}", tailscale, config._tailnet_base)
    } else if let Some(ref ip) = host_config.ip {
        ip.clone()
    } else {
        anyhow::bail!(
            "Host '{}' has no IP or Tailscale address configured",
            hostname
        );
    };

    println!("Syncing with {} ({})...", hostname, target_host);
    println!();

    // Create SSH connection
    let ssh = SshConnection::new(&target_host)
        .with_context(|| format!("Failed to connect to {}", target_host))?;

    if pull {
        pull_from_remote(&ssh, hostname)?;
    } else {
        push_to_remote(&ssh, hostname)?;
    }

    Ok(())
}

/// Push data to remote halvor installation
fn push_to_remote(ssh: &SshConnection, _hostname: &str) -> Result<()> {
    println!("Pushing data to remote halvor installation...");

    // Export encrypted data
    let encrypted_data = db::export_encrypted_data()?;
    println!(
        "  Exported {} bytes of encrypted data",
        encrypted_data.len()
    );

    // Get remote halvor database path
    let remote_db_path = get_remote_db_path(ssh)?;
    println!("  Remote database: {}", remote_db_path);

    // Create temp file locally with the data
    let temp_file =
        std::env::temp_dir().join(format!("hal-sync-{}.json", chrono::Utc::now().timestamp()));
    std::fs::write(&temp_file, &encrypted_data).context("Failed to write temp sync file")?;

    // Copy encrypted data to remote
    let remote_temp = format!("/tmp/hal-sync-{}.json", chrono::Utc::now().timestamp());
    copy_file_to_remote(ssh, &temp_file, &remote_temp)?;
    println!("  Copied encrypted data to remote");

    // Import on remote - we'll need to add db import command or use a different approach
    // For now, let's use a base64 encoded approach via stdin
    let data_base64 = STANDARD.encode(&encrypted_data);
    let import_script = format!(
        r#"
        if ! command -v hal >/dev/null 2>&1; then
            echo "Error: hal not found on remote host"
            exit 1
        fi
        
        # Decode and import encrypted data
        echo '{}' | base64 -d > {} && halvor db import {} && rm -f {} || {{
            echo "Failed to import encrypted data"
            exit 1
        }}
        "#,
        data_base64, remote_temp, remote_temp, remote_temp
    );

    ssh.execute_shell(&import_script)
        .context("Failed to import data on remote")?;

    // Copy encryption key to remote (if not already present)
    // Note: Encryption key sync requires manual setup for security
    println!("  Note: Encryption key sync requires manual setup");

    // Clean up local temp file
    std::fs::remove_file(&temp_file).ok();

    println!("✓ Data pushed successfully");

    Ok(())
}

/// Pull data from remote halvor installation
fn pull_from_remote(ssh: &SshConnection, _hostname: &str) -> Result<()> {
    println!("Pulling data from remote halvor installation...");

    // Get remote halvor database path
    let remote_db_path = get_remote_db_path(ssh)?;
    println!("  Remote database: {}", remote_db_path);

    // Export from remote
    let export_script = r#"
        if ! command -v halvor >/dev/null 2>&1; then
            echo "Error: halvor not found on remote host"
            exit 1
        fi
        
        halvor db export
    "#;

    let output = ssh
        .execute_shell(export_script)
        .context("Failed to export data from remote")?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to export data from remote: {}",
            bytes_to_string(&output.stderr)
        );
    }

    let encrypted_data = output.stdout;
    println!(
        "  Received {} bytes of encrypted data",
        encrypted_data.len()
    );

    // Import locally
    db::import_encrypted_data(&encrypted_data)?;
    println!("  Imported encrypted data");

    // Note: Encryption key sync requires manual setup for security
    println!("  Note: Encryption key sync requires manual setup");

    println!("✓ Data pulled successfully");

    Ok(())
}

/// Get the remote halvor database path
fn get_remote_db_path(ssh: &SshConnection) -> Result<String> {
    let script = r#"
        if command -v halvor >/dev/null 2>&1; then
            halvor config db-path 2>/dev/null || echo "$HOME/.config/halvor/halvor.db"
        else
            echo "$HOME/.config/halvor/halvor.db"
        fi
    "#;

    let output = ssh
        .execute_shell(script)
        .context("Failed to get remote database path")?;

    if !output.status.success() {
        anyhow::bail!("Failed to get remote database path");
    }

    let path = bytes_to_string(&output.stdout);
    Ok(path)
}

/// Copy a file to remote via SSH
fn copy_file_to_remote(ssh: &SshConnection, local_path: &PathBuf, remote_path: &str) -> Result<()> {
    use std::process::Command;

    let host = &ssh.host;
    let mut scp_args = vec!["-o".to_string(), "StrictHostKeyChecking=no".to_string()];

    if ssh.use_key_auth {
        scp_args.extend([
            "-o".to_string(),
            "PreferredAuthentications=publickey".to_string(),
            "-o".to_string(),
            "PasswordAuthentication=no".to_string(),
        ]);
    }

    scp_args.push(format!("{}:{}", local_path.display(), remote_path));
    scp_args.insert(0, host.clone());

    let status = Command::new("scp")
        .args(&scp_args)
        .status()
        .context("Failed to execute scp")?;

    if !status.success() {
        anyhow::bail!("Failed to copy file to remote");
    }

    Ok(())
}
