use crate::config::EnvConfig;
use crate::utils::exec::CommandExecutor;
use crate::utils::service::{DockerOps, FileOps, ServiceContext};
use anyhow::{Context, Result};
use std::time::SystemTime;

// New host-level backup functions
pub fn backup_host(hostname: &str, config: &EnvConfig) -> Result<()> {
    let ctx = ServiceContext::new(hostname, config)?;
    let backup_base = ctx.backup_path()?;

    ctx.print_start("Backing up all Docker volumes");
    perform_backup(ctx.exec(), hostname, backup_base)?;
    ctx.print_complete("Backup");

    Ok(())
}

pub fn list_backups(hostname: &str, config: &EnvConfig) -> Result<()> {
    let ctx = ServiceContext::new(hostname, config)?;
    let backup_base = ctx.backup_path()?;

    println!("Listing backups for {} ({})...", hostname, ctx.target_host);
    println!();

    list_backup_directories(ctx.exec(), backup_base)?;

    Ok(())
}

/// Backup a specific service (e.g., portainer, sonarr)
pub fn backup_service(hostname: &str, service: &str, config: &EnvConfig) -> Result<()> {
    let ctx = ServiceContext::new(hostname, config)?;
    let backup_base = ctx.backup_path()?;

    println!("Backing up service '{}' on {}...", service, hostname);
    println!();

    // Create service-specific backup directory
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let service_backup_dir = format!("{}/{}/{}", backup_base, service, timestamp);

    // Ensure directory exists
    ctx.exec().mkdir_p(&service_backup_dir)?;

    // Find containers for this service
    let containers = ctx.exec().list_containers()?;
    let service_containers: Vec<String> = containers
        .into_iter()
        .filter(|c| c.to_lowercase().contains(&service.to_lowercase()))
        .collect();

    if service_containers.is_empty() {
        anyhow::bail!("No containers found for service '{}'", service);
    }

    println!(
        "Found {} container(s) for service '{}':",
        service_containers.len(),
        service
    );
    for container in &service_containers {
        println!("  - {}", container);
    }
    println!();

    // Backup volumes for each container
    for container in &service_containers {
        println!("Backing up container: {}", container);

        // Get volumes for this container
        let volumes = get_container_volumes(ctx.exec(), container)?;
        for volume in &volumes {
            println!("  Backing up volume: {}", volume);
            if let Err(e) = ctx.exec().backup_volume(volume, &service_backup_dir) {
                println!("    ✗ Failed: {}", e);
            } else {
                println!("    ✓ Backed up");
            }
        }

        // Get bind mounts
        let mounts = ctx.exec().get_bind_mounts(container)?;
        for mount in &mounts {
            if ctx.exec().is_dir(mount)? {
                let mount_name = mount
                    .split('/')
                    .last()
                    .unwrap_or("unknown")
                    .replace('/', "_");
                let backup_name = format!("{}_{}", container, mount_name);
                println!("  Backing up bind mount: {}", mount);

                let backup_cmd = format!(
                    "docker run --rm -v {}:/data:ro -v {}:/backup alpine tar czf /backup/{}.tar.gz -C /data .",
                    mount, service_backup_dir, backup_name
                );
                let output = ctx.exec().execute_shell(&backup_cmd)?;
                if output.status.success() {
                    println!("    ✓ Backed up");
                } else {
                    println!("    ✗ Failed");
                }
            }
        }
    }

    // Create zip file
    let zip_path = format!("{}/{}_{}.zip", backup_base, service, timestamp);
    println!();
    println!("Creating zip archive: {}", zip_path);

    let zip_cmd = format!("cd {} && zip -r {} {}", backup_base, zip_path, service);
    let zip_output = ctx.exec().execute_shell(&zip_cmd)?;

    if zip_output.status.success() {
        println!("✓ Backup complete: {}", zip_path);
    } else {
        anyhow::bail!("Failed to create zip archive");
    }

    Ok(())
}

/// Backup config to env location
pub fn backup_to_env(hostname: &str, _service: Option<&str>, _config: &EnvConfig) -> Result<()> {
    use crate::config::service;

    println!("Backing up config to .env file...");
    println!();

    if hostname != "localhost" {
        service::backup_host_config_to_env(hostname)?;
    } else {
        service::backup_all_to_env()?;
    }

    println!("✓ Config backed up to .env");
    Ok(())
}

/// Interactive backup selection
pub fn backup_interactive(hostname: &str, config: &EnvConfig) -> Result<()> {
    let ctx = ServiceContext::new(hostname, config)?;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Interactive Backup Selection");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Select services to backup (Space to toggle, Enter to confirm):");
    println!();

    // Get running containers
    let containers = ctx.exec().list_containers()?;
    let running_containers: Vec<String> = containers
        .into_iter()
        .filter(|c| ctx.exec().is_container_running(c).unwrap_or(false))
        .collect();

    if running_containers.is_empty() {
        println!("No running containers found.");
        return Ok(());
    }

    // For now, just backup all running services
    // TODO: Implement interactive selection with dialoguer or similar
    println!("Backing up all running services...");
    println!();

    for container in &running_containers {
        // Extract service name from container name
        let service_name = container.split('-').next().unwrap_or(container);
        if let Err(e) = backup_service(hostname, service_name, config) {
            println!("Failed to backup {}: {}", container, e);
        }
    }

    // Also backup config if requested
    println!();
    println!("Backing up config to .env...");
    backup_to_env(hostname, None, config)?;

    Ok(())
}

/// Restore a specific service
pub fn restore_service(
    hostname: &str,
    service: &str,
    backup_timestamp: Option<&str>,
    config: &EnvConfig,
) -> Result<()> {
    let ctx = ServiceContext::new(hostname, config)?;
    let backup_base = ctx.backup_path()?;

    // List available backups for this service
    let list_cmd = format!("ls -1d {}/{}/* 2>/dev/null | sort -r", backup_base, service);
    let list_output = ctx.exec().execute_shell(&list_cmd)?;
    let backups_str = crate::utils::bytes_to_string(&list_output.stdout);
    let backups: Vec<&str> = backups_str
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();

    if backups.is_empty() {
        anyhow::bail!("No backups found for service '{}'", service);
    }

    let backup_to_restore = if let Some(timestamp) = backup_timestamp {
        backups
            .iter()
            .find(|b| b.contains(timestamp))
            .ok_or_else(|| anyhow::anyhow!("Backup with timestamp '{}' not found", timestamp))?
            .to_string()
    } else {
        // Use most recent backup
        backups[0].to_string()
    };

    println!(
        "Restoring service '{}' from backup: {}",
        service, backup_to_restore
    );
    println!();

    // Extract zip if needed
    if backup_to_restore.ends_with(".zip") {
        let extract_cmd = format!(
            "cd {} && unzip -o {} -d {}",
            backup_base, backup_to_restore, service
        );
        ctx.exec().execute_shell(&extract_cmd)?;
    }

    // Restore volumes from the backup directory
    let backup_dir = if backup_to_restore.ends_with(".zip") {
        format!("{}/{}", backup_base, service)
    } else {
        backup_to_restore
    };

    let restore_cmd = format!("ls -1 {}/*.tar.gz 2>/dev/null", backup_dir);
    let restore_output = ctx.exec().execute_shell(&restore_cmd)?;
    let restore_files = crate::utils::bytes_to_string(&restore_output.stdout);

    for file in restore_files.lines().filter(|l| !l.trim().is_empty()) {
        let vol_name = file
            .split('/')
            .last()
            .unwrap_or("")
            .trim_end_matches(".tar.gz");

        if !vol_name.is_empty() {
            println!("Restoring volume: {}", vol_name);
            ctx.exec().restore_volume(vol_name, &backup_dir)?;
        }
    }

    println!("✓ Service '{}' restored", service);
    Ok(())
}

/// Restore from env location
pub fn restore_from_env(hostname: &str, _service: Option<&str>, config: &EnvConfig) -> Result<()> {
    use crate::config::service;

    println!("Restoring config from .env file...");
    println!();

    if hostname != "localhost" {
        service::commit_host_config_to_db(hostname)?;
    } else {
        service::commit_all_to_db()?;
    }

    println!("✓ Config restored from .env");
    Ok(())
}

/// Interactive restore selection
pub fn restore_interactive(hostname: &str, config: &EnvConfig) -> Result<()> {
    let ctx = ServiceContext::new(hostname, config)?;
    let backup_base = ctx.backup_path()?;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Interactive Restore Selection");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // List available services with backups
    let list_cmd = format!("ls -1d {}/*/ 2>/dev/null | xargs -n1 basename", backup_base);
    let list_output = ctx.exec().execute_shell(&list_cmd)?;
    let services_str = crate::utils::bytes_to_string(&list_output.stdout);
    let services: Vec<&str> = services_str
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();

    if services.is_empty() {
        println!("No service backups found.");
        return Ok(());
    }

    println!("Available services with backups:");
    for (i, service) in services.iter().enumerate() {
        println!("  {}. {}", i + 1, service);
    }
    println!();

    // For now, just list - TODO: Implement interactive selection
    println!("Use: halvor restore <service> [-H <hostname>] [--backup <timestamp>]");
    println!("Example: halvor restore portainer -H bellerophon");

    Ok(())
}

// Helper function to get volumes for a container
fn get_container_volumes<E: CommandExecutor + DockerOps>(
    exec: &E,
    container: &str,
) -> Result<Vec<String>> {
    let inspect_cmd = format!(
        r#"docker inspect {} --format '{{{{range .Mounts}}}}{{{{if eq .Type "volume"}}}}{{{{.Name}}}}{{{{end}}}}{{{{end}}}}'"#,
        container
    );
    let output = exec.execute_shell(&inspect_cmd)?;
    let volumes_str = crate::utils::bytes_to_string(&output.stdout);
    let volumes: Vec<String> = volumes_str
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|s| s.to_string())
        .collect();
    Ok(volumes)
}

// ... existing code continues below ...

pub fn restore_host(hostname: &str, backup_name: Option<&str>, config: &EnvConfig) -> Result<()> {
    let ctx = ServiceContext::new(hostname, config)?;
    let backup_base = ctx.backup_path()?;

    if let Some(backup) = backup_name {
        ctx.print_start(&format!("Restoring {} from backup '{}'", hostname, backup));
        perform_restore(ctx.exec(), hostname, backup_base, backup)?;
        ctx.print_complete("Restore");
    } else {
        // Interactive restore
        restore_interactive(hostname, config)?;
    }

    Ok(())
}

// ... rest of existing functions (perform_backup, perform_restore, etc.) ...

fn perform_backup<E: CommandExecutor + DockerOps + FileOps>(
    exec: &E,
    hostname: &str,
    backup_base: &str,
) -> Result<()> {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let timestamp_str = timestamp.to_string();
    let backup_dir = format!("{}/{}", backup_base, timestamp_str);

    println!("Creating backup directory: {}", backup_dir);
    exec.mkdir_p(&backup_dir)?;

    println!();
    println!("=== Stopping all containers ===");

    let running_containers = exec.stop_all_containers()?;
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
    let volumes = exec.list_volumes()?;

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
            if let Err(e) = exec.backup_volume(vol, &backup_dir) {
                println!("    ✗ Failed to backup volume: {} - {}", vol, e);
            } else {
                println!("    ✓ Volume {} backed up", vol);
            }
        }
    }

    println!();
    println!("=== Backing up bind mounts from containers ===");

    // Get all containers
    let containers = exec.list_containers()?;

    if containers.is_empty() {
        println!("No containers found");
    } else {
        for container in &containers {
            // Get bind mounts for this container
            let mounts = exec.get_bind_mounts(container)?;

            for mount_path in &mounts {
                // Check if mount path is a directory using native Rust
                if exec.is_dir(mount_path)? {
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
        volumes.join("\n")
    );

    let metadata_path = format!("{}/metadata.txt", backup_dir);
    exec.write_file(&metadata_path, metadata.as_bytes())?;

    println!();
    println!("=== Starting containers ===");

    if !running_containers.is_empty() {
        println!("Starting containers...");
        exec.start_containers(&running_containers)?;
        println!("✓ Containers started");
    } else {
        println!("✓ No containers to start");
    }

    println!();
    println!("=== Backup Summary ===");
    println!("Backup location: {}", backup_dir);
    println!("Host: {}", hostname);
    println!("Timestamp: {}", timestamp_str);

    Ok(())
}

fn perform_restore<E: CommandExecutor + DockerOps + FileOps>(
    exec: &E,
    hostname: &str,
    backup_base: &str,
    backup_name: &str,
) -> Result<()> {
    let backup_dir = format!("{}/{}", backup_base, backup_name);

    println!("Restoring from: {}", backup_dir);

    // Check if backup directory exists
    if !exec.is_dir(&backup_dir)? {
        anyhow::bail!("Backup directory not found: {}", backup_dir);
    }

    println!();
    println!("=== Stopping all containers ===");

    let running_containers = exec.stop_all_containers()?;
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
    let files_str = crate::utils::bytes_to_string(&list_files.stdout);
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
        if let Err(e) = exec.restore_volume(vol_name, &backup_dir) {
            println!("  ✗ Failed to restore volume: {} - {}", vol_name, e);
        } else {
            println!("  ✓ Restored volume: {}", vol_name);
        }
    }

    println!();
    println!("=== Starting containers ===");

    if !running_containers.is_empty() {
        println!("Starting containers...");
        exec.start_containers(&running_containers)?;
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

fn list_backup_directories<E: CommandExecutor>(exec: &E, backup_base: &str) -> Result<()> {
    let list_cmd = format!("ls -1td {}/*/ 2>/dev/null | head -10", backup_base);
    let list_output = exec.execute_shell(&list_cmd)?;
    let dirs_str = crate::utils::bytes_to_string(&list_output.stdout);
    let dirs: Vec<&str> = dirs_str.lines().filter(|l| !l.trim().is_empty()).collect();

    if dirs.is_empty() {
        println!("No backups found in {}", backup_base);
        return Ok(());
    }

    println!("Available backups:");
    for dir in &dirs {
        let dir_name = dir.split('/').last().unwrap_or("unknown");
        println!("  - {}", dir_name);
    }

    Ok(())
}
