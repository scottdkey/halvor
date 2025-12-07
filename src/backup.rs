use crate::exec::{CommandExecutor, Executor};
use crate::{config::EnvConfig, docker, tailscale};
use anyhow::{Context, Result};
use std::time::SystemTime;

// New host-level backup functions
pub fn backup_host(hostname: &str, config: &EnvConfig) -> Result<()> {
    // Create executor - it automatically determines if execution should be local or remote
    let exec = Executor::new(hostname, config)?;
    let target_host = exec.target_host(hostname, config)?;
    let is_local = exec.is_local();

    // Get backup path from config (required)
    let host_config = tailscale::get_host_config(config, hostname)?;

    let backup_base = host_config.backup_path.as_ref().with_context(|| {
        format!(
            "Backup path not configured for {}\n\nAdd to .env:\n  HOST_{}_BACKUP_PATH=\"/path/to/backups/{}\"",
            hostname,
            hostname.to_uppercase(),
            hostname
        )
    })?;

    if is_local {
        println!("Backing up all Docker volumes locally on {}...", hostname);
    } else {
        println!(
            "Backing up all Docker volumes on {} ({})...",
            hostname, target_host
        );
    }
    println!();

    perform_backup(&exec, hostname, backup_base)?;

    println!();
    println!("✓ Backup complete for {}", hostname);

    Ok(())
}

pub fn list_backups(hostname: &str, config: &EnvConfig) -> Result<()> {
    // Create executor - it automatically determines if execution should be local or remote
    let exec = Executor::new(hostname, config)?;
    let target_host = exec.target_host(hostname, config)?;
    let is_local = exec.is_local();

    // Get backup path from config (required)
    let host_config = tailscale::get_host_config(config, hostname)?;

    let backup_base = host_config.backup_path.as_ref().with_context(|| {
        format!(
            "Backup path not configured for {}\n\nAdd to .env:\n  HOST_{}_BACKUP_PATH=\"/path/to/backups/{}\"",
            hostname,
            hostname.to_uppercase(),
            hostname
        )
    })?;

    if is_local {
        println!("Listing backups locally for {}...", hostname);
    } else {
        println!("Listing backups for {} ({})...", hostname, target_host);
    }
    println!();

    list_backup_directories(&exec, backup_base)?;

    Ok(())
}

pub fn restore_host(hostname: &str, backup_name: Option<&str>, config: &EnvConfig) -> Result<()> {
    // Create executor - it automatically determines if execution should be local or remote
    let exec = Executor::new(hostname, config)?;
    let _target_host = exec.target_host(hostname, config)?;
    let _is_local = exec.is_local();

    // Get backup path from config (required)
    let host_config = tailscale::get_host_config(config, hostname)?;

    let backup_base = host_config.backup_path.as_ref().with_context(|| {
        format!(
            "Backup path not configured for {}\n\nAdd to .env:\n  HOST_{}_BACKUP_PATH=\"/path/to/backups/{}\"",
            hostname,
            hostname.to_uppercase(),
            hostname
        )
    })?;

    if let Some(backup) = backup_name {
        println!("Restoring {} from backup '{}'...", hostname, backup);
        println!();

        perform_restore(&exec, hostname, backup_base, backup)?;

        println!();
        println!("✓ Restore complete for {}", hostname);
    } else {
        // List backups and prompt
        println!("No backup name specified. Available backups:");
        println!();
        list_backups(hostname, config)?;
        println!();
        println!(
            "Use: hal backup {} restore --backup <backup-name>",
            hostname
        );
    }

    Ok(())
}

fn perform_backup<E: CommandExecutor>(exec: &E, hostname: &str, backup_base: &str) -> Result<()> {
    let datetime = chrono::DateTime::<chrono::Utc>::from(SystemTime::now());
    let timestamp_str = datetime.format("%Y%m%d_%H%M%S").to_string();
    let backup_dir = format!("{}/{}", backup_base, timestamp_str);

    println!("=== Host Backup Configuration ===");
    println!("Host: {}", hostname);
    println!("Backup Directory: {}", backup_dir);
    println!();

    // Check if backup base directory exists (parent of backup_base) using native Rust
    if let Some(parent) = std::path::Path::new(backup_base).parent() {
        let parent_str = parent.to_string_lossy();
        if !exec.is_directory(&parent_str)? {
            anyhow::bail!(
                "Error: Backup base directory {} does not exist or is not mounted\nMake sure SMB mount is set up: hal smb {} setup",
                parent_str,
                hostname
            );
        }
    }

    // Create backup base directory if it doesn't exist using native Rust
    if !exec.is_directory(backup_base)? {
        println!("Creating backup base directory: {}", backup_base);
        exec.mkdir_p(backup_base)?;
        println!("✓ Created backup base directory");
    }

    // Create backup directory
    exec.mkdir_p(&backup_dir)?;
    println!("✓ Created backup directory: {}", backup_dir);

    println!();
    println!("=== Stopping all containers ===");

    let running_containers = docker::stop_all_containers(exec)?;
    if !running_containers.is_empty() {
        println!(
            "Stopped {} running container(s)...",
            running_containers.len()
        );
        println!("✓ All containers stopped");
    } else {
        println!("✓ No running containers to stop");
    }

    println!();
    println!("=== Backing up Docker volumes ===");

    // Get all Docker volumes
    let volumes = docker::list_volumes(exec)?;

    if volumes.is_empty() {
        println!("No Docker volumes found");
    } else {
        println!("Found {} volume(s) to backup:", volumes.len());
        for vol in &volumes {
            println!("  - {}", vol);
        }
        println!();

        // Backup each volume
        for vol in &volumes {
            println!("  Backing up volume: {}", vol);
            if let Err(e) = docker::backup_volume(exec, vol, &backup_dir) {
                println!("    ✗ Failed to backup volume: {} - {}", vol, e);
            } else {
                println!("    ✓ Volume {} backed up", vol);
            }
        }
    }

    println!();
    println!("=== Backing up bind mounts from containers ===");

    // Get all containers
    let containers = docker::list_containers(exec)?;

    if containers.is_empty() {
        println!("No containers found");
    } else {
        for container in &containers {
            // Get bind mounts for this container
            let mounts = docker::get_bind_mounts(exec, container)?;

            for mount_path in &mounts {
                // Check if mount path is a directory using native Rust
                if exec.is_directory(mount_path)? {
                    let mount_name = mount_path
                        .split('/')
                        .last()
                        .unwrap_or("unknown")
                        .replace('/', "_");
                    let backup_name = format!("{}_{}", container, mount_name);
                    println!("  Backing up bind mount from {}: {}", container, mount_path);

                    // Use docker::backup_volume logic but for bind mounts
                    let backup_cmd = format!(
                        "docker run --rm -v {}:/data:ro -v {}:/backup alpine tar czf /backup/{}.tar.gz -C /data .",
                        mount_path, backup_dir, backup_name
                    );
                    let backup_output = exec.execute_shell(&backup_cmd)?;
                    if backup_output.status.success() {
                        println!(
                            "    ✓ Bind mount {} backed up as {}.tar.gz",
                            mount_path, backup_name
                        );
                    } else {
                        // Try with sudo
                        let sudo_backup_cmd = format!(
                            "sudo docker run --rm -v {}:/data:ro -v {}:/backup alpine tar czf /backup/{}.tar.gz -C /data .",
                            mount_path, backup_dir, backup_name
                        );
                        let sudo_output = exec.execute_shell(&sudo_backup_cmd)?;
                        if sudo_output.status.success() {
                            println!(
                                "    ✓ Bind mount {} backed up as {}.tar.gz",
                                mount_path, backup_name
                            );
                        } else {
                            println!("    ✗ Failed to backup bind mount: {}", mount_path);
                        }
                    }
                }
            }
        }
    }

    // Create metadata file
    let metadata = format!(
        "Host: {}\nTimestamp: {}\nDate: {}\nVolume Count: {}\nVolumes:\n{}",
        hostname,
        timestamp_str,
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        volumes.len(),
        volumes
            .iter()
            .map(|v| format!("  - {}", v))
            .collect::<Vec<_>>()
            .join("\n")
    );
    exec.write_file(
        &format!("{}/backup-info.txt", backup_dir),
        metadata.as_bytes(),
    )?;
    println!("✓ Created backup metadata");

    println!();
    println!("=== Starting containers ===");

    if !running_containers.is_empty() {
        println!("Starting containers...");
        docker::start_containers(exec, &running_containers)?;
        println!("✓ Containers started");
    } else {
        println!("✓ No containers to start");
    }

    println!();
    println!("=== Backup Summary ===");
    println!("Backup location: {}", backup_dir);
    println!("Backup name: {}", timestamp_str);
    println!("Volumes backed up: {}", volumes.len());

    let list_output = exec.execute_shell(&format!("ls -lh {}", backup_dir))?;
    if list_output.status.success() {
        let list_str = String::from_utf8_lossy(&list_output.stdout);
        for line in list_str.lines().skip(1) {
            println!("{}", line);
        }
    }

    Ok(())
}

fn list_backup_directories<E: CommandExecutor>(exec: &E, backup_base: &str) -> Result<()> {
    println!("=== Available Backups ===");

    // Check if backup directory exists using native Rust
    if !exec.is_directory(backup_base)? {
        anyhow::bail!(
            "Error: Backup directory {} does not exist or is not mounted",
            backup_base
        );
    }

    // List backup directories using native Rust
    let backup_dirs = exec.list_directory(backup_base)?;
    let backup_dirs: Vec<String> = backup_dirs
        .into_iter()
        .map(|name| format!("{}/{}", backup_base, name))
        .filter(|path| exec.is_directory(path).unwrap_or(false))
        .collect();

    if backup_dirs.is_empty() {
        println!("No backups found");
        return Ok(());
    }

    println!("Found {} backup(s):", backup_dirs.len());
    println!();

    for backup_dir in &backup_dirs {
        let backup_name = backup_dir.split('/').last().unwrap_or("unknown");

        // Get backup date
        let stat_cmd = format!(
            r#"stat -c %y "{}" 2>/dev/null || stat -f %Sm "{}" 2>/dev/null || echo "unknown""#,
            backup_dir, backup_dir
        );
        let date_output = exec.execute_shell(&stat_cmd)?;
        let backup_date = String::from_utf8_lossy(&date_output.stdout)
            .trim()
            .to_string();

        println!("  - {}", backup_name);
        println!("    Date: {}", backup_date);

        // Read backup info if it exists
        let info_path = format!("{}/backup-info.txt", backup_dir);
        let info_exists = exec.file_exists(&info_path)?;
        if info_exists {
            let info_output = exec.execute_shell(&format!("cat {}", info_path))?;
            if info_output.status.success() {
                let info_str = String::from_utf8_lossy(&info_output.stdout);
                println!("    Info:");
                for line in info_str.lines() {
                    println!("      {}", line);
                }
            }
        }
        println!();
    }

    Ok(())
}

fn perform_restore<E: CommandExecutor>(
    exec: &E,
    hostname: &str,
    backup_base: &str,
    backup_name: &str,
) -> Result<()> {
    let backup_dir = format!("{}/{}", backup_base, backup_name);

    println!("=== Restore Configuration ===");
    println!("Host: {}", hostname);
    println!("Backup: {}", backup_name);
    println!("Backup Directory: {}", backup_dir);
    println!();

    // Check if backup base directory exists (parent of backup_base) using native Rust
    if let Some(parent) = std::path::Path::new(backup_base).parent() {
        let parent_str = parent.to_string_lossy();
        if !exec.is_directory(&parent_str)? {
            anyhow::bail!(
                "Error: Backup base directory {} does not exist or is not mounted\nMake sure SMB mount is set up: hal smb <hostname> setup",
                parent_str
            );
        }
    }

    // Create backup base directory if it doesn't exist using native Rust
    if !exec.is_directory(backup_base)? {
        exec.mkdir_p(backup_base)?;
        println!("Created backup base directory: {}", backup_base);
    }

    // Check if backup exists using native Rust
    if !exec.is_directory(&backup_dir)? {
        println!("Error: Backup directory does not exist: {}", backup_dir);
        println!("Available backups:");
        let dirs = exec.list_directory(backup_base)?;
        if dirs.is_empty() {
            println!("  (none)");
        } else {
            for dir in dirs {
                println!("  {}", dir);
            }
        }
        anyhow::bail!("Backup directory does not exist: {}", backup_dir);
    }

    println!();
    println!("=== Stopping all containers ===");

    let running_containers = docker::stop_all_containers(exec)?;
    if !running_containers.is_empty() {
        println!(
            "Stopped {} running container(s)...",
            running_containers.len()
        );
        println!("✓ All containers stopped");
    } else {
        println!("✓ No running containers to stop");
    }

    println!();
    println!("=== Restoring Docker volumes ===");

    // List backup files
    let list_files = exec.execute_shell(&format!(
        "ls -1 {}/*.tar.gz 2>/dev/null || true",
        backup_dir
    ))?;
    let files_str = String::from_utf8_lossy(&list_files.stdout);
    let backup_files: Vec<&str> = files_str
        .lines()
        .filter(|l| !l.trim().is_empty() && l.ends_with(".tar.gz"))
        .collect();

    for backup_file in &backup_files {
        // Extract volume name from filename (remove path and .tar.gz extension)
        let vol_name = backup_file
            .split('/')
            .last()
            .unwrap_or("")
            .trim_end_matches(".tar.gz");

        if vol_name.is_empty() {
            continue;
        }

        println!("Restoring volume: {}", vol_name);

        // Restore volume using docker module
        if let Err(e) = docker::restore_volume(exec, vol_name, &backup_dir) {
            println!("  ✗ Failed to restore volume: {} - {}", vol_name, e);
        } else {
            println!("  ✓ Restored volume: {}", vol_name);
        }
    }

    println!();
    println!("=== Starting containers ===");

    if !running_containers.is_empty() {
        println!("Starting containers...");
        docker::start_containers(exec, &running_containers)?;
        println!("✓ Containers started");
    } else {
        println!("✓ No containers to start");
    }

    println!();
    println!("=== Restore Summary ===");
    println!("Restored from: {}", backup_dir);
    println!("Host: {}", hostname);

    Ok(())
}
