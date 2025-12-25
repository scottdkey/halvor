use crate::config::EnvConfig;
use crate::utils::exec::{CommandExecutor, Executor};
use anyhow::Result;

pub fn setup_smb_mounts(hostname: &str, config: &EnvConfig) -> Result<()> {
    // Create executor - it automatically determines if execution should be local or remote
    let exec = Executor::new(hostname, config)?;
    let is_local = exec.is_local();
    
    // Get the actual hostname for display - detect current machine if local
    let display_hostname = if is_local && (hostname == "localhost" || hostname == "127.0.0.1") {
        // Try to detect current hostname
        crate::config::service::get_current_hostname()
            .ok()
            .unwrap_or_else(|| hostname.to_string())
    } else {
        hostname.to_string()
    };
    
    let target_host = exec.target_host(hostname, config)?;

    if is_local {
        println!("Setting up SMB mounts locally on {}...", display_hostname);
    } else {
        println!("Setting up SMB mounts on {} ({})...", hostname, target_host);
    }
    println!();

    // Execute setup using Rust-native operations
    setup_smb_mounts_remote(&exec, config)?;

    println!();
    println!("✓ SMB mount setup complete for {}", display_hostname);

    Ok(())
}

pub fn uninstall_smb_mounts(hostname: &str, config: &EnvConfig) -> Result<()> {
    // Create executor - it automatically determines if execution should be local or remote
    let exec = Executor::new(hostname, config)?;
    let target_host = exec.target_host(hostname, config)?;
    let is_local = exec.is_local();

    if is_local {
        println!("Uninstalling SMB mounts locally on {}...", hostname);
    } else {
        println!(
            "Uninstalling SMB mounts on {} ({})...",
            hostname, target_host
        );
    }
    println!();

    // Execute uninstall using Rust-native operations
    uninstall_smb_mounts_remote(&exec, config)?;

    println!();
    println!("✓ SMB mounts removed from {}", hostname);

    Ok(())
}

fn setup_smb_mounts_remote<E: CommandExecutor>(exec: &E, config: &EnvConfig) -> Result<()> {
    println!("=== SMB Configuration ===");
    println!("Configuration loaded from .env file");
    println!(
        "Number of SMB servers configured: {}",
        config.smb_servers.len()
    );

    // Add configuration summary
    for (server_name, server_config) in &config.smb_servers {
        println!(
            "  - {}: {} ({} share(s))",
            server_name,
            server_config.host,
            server_config.shares.len()
        );
        for share in &server_config.shares {
            println!("    └─ {} -> /mnt/smb/{}/{}", share, server_name, share);
        }
    }
    println!();

    // Install SMB client
    install_smb_client(exec)?;

    // Clean up old mounts
    cleanup_old_mounts(exec)?;

    // Create mount directory
    println!("=== Creating SMB mount directory ===");
    // For system directories like /mnt, we need sudo (interactive for password prompt)
    exec.execute_interactive("sudo", &["mkdir", "-p", "/mnt/smb"])?;
    println!("✓ Mount directory created");
    println!();

    // Mount each share
    for (server_name, server_config) in &config.smb_servers {
        for share_name in &server_config.shares {
            let mount_point = format!("/mnt/smb/{}/{}", server_name, share_name);
            let share_path = format!("//{}/{}", server_config.host, share_name);

            setup_smb_share(
                exec,
                server_name,
                share_name,
                &share_path,
                &mount_point,
                server_config,
            )?;
        }
    }

    println!();
    println!("=== SMB setup complete ===");

    Ok(())
}

fn install_smb_client<E: CommandExecutor>(exec: &E) -> Result<()> {
    println!("=== Installing SMB client ===");

    // Check if mount.cifs exists
    if exec.check_command_exists("mount.cifs")? {
        println!("✓ SMB client already installed");
        return Ok(());
    }

    // Detect package manager and install
    let pkg_mgr = crate::utils::exec::PackageManager::detect(exec)?;
    println!("Detected package manager: {}", pkg_mgr.display_name());
    pkg_mgr.install_package(exec, "cifs-utils")?;

    println!("✓ SMB client installed");
    Ok(())
}

fn cleanup_old_mounts<E: CommandExecutor>(exec: &E) -> Result<()> {
    println!("=== Cleaning up old mounts ===");

    // Check if /mnt/smb exists before trying to list it
    // If it doesn't exist, there's nothing to clean up
    let smb_dir_exists = exec.is_directory("/mnt/smb").unwrap_or(false);
    if !smb_dir_exists {
        println!("Mount directory /mnt/smb doesn't exist yet, nothing to clean up");
        println!();
        return Ok(());
    }

    // List directories in /mnt/smb - handle gracefully if directory doesn't exist
    let dirs = exec.list_directory("/mnt/smb")
        .unwrap_or_else(|_| {
            // Directory might have been removed between check and list, or check might have been wrong
            println!("Mount directory /mnt/smb doesn't exist, nothing to clean up");
            Vec::new()
        });
    for server_dir in dirs {
        let server_dir = server_dir.trim();
        if server_dir.is_empty() {
            continue;
        }

        let full_path = format!("/mnt/smb/{}", server_dir);

        // Check if it's a mount point
        let mountpoint_check = exec.execute_simple("mountpoint", &["-q", &full_path]);
        if let Ok(output) = mountpoint_check {
            if output.status.success() {
                println!("Found old mount at {}, unmounting...", full_path);
                exec.execute_simple("sudo", &["umount", &full_path]).ok();
                remove_fstab_entry(exec, &full_path)?;
                println!("✓ Cleaned up old mount at {}", full_path);
            }
        }
    }

    println!();
    Ok(())
}

fn setup_smb_share<E: CommandExecutor>(
    exec: &E,
    server_name: &str,
    share_name: &str,
    share_path: &str,
    mount_point: &str,
    server_config: &crate::config::SmbServerConfig,
) -> Result<()> {
    println!();
    println!("=== Setting up {} - {} ===", server_name, share_name);
    println!("Configuration:");
    println!("  Server: {}", server_name);
    println!("  Host: {}", server_config.host);
    println!("  Share: {}", share_name);
    println!("  Mount Point: {}", mount_point);
    println!(
        "  Username: {}",
        server_config
            .username
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("(not set)")
    );
    println!(
        "  Options: {}",
        server_config
            .options
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("(none)")
    );

    // Validate credentials
    let username = server_config.username.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "No username configured for {} - {}",
            server_name,
            share_name
        )
    })?;

    let password = server_config.password.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "No password configured for {} - {}",
            server_name,
            share_name
        )
    })?;

    // Create mount point
    // For system directories under /mnt, we need sudo (interactive for password prompt)
    exec.execute_interactive("sudo", &["mkdir", "-p", mount_point])?;

    // Check if already mounted
    let mountpoint_check = exec.execute_simple("mountpoint", &["-q", mount_point]);
    if let Ok(output) = mountpoint_check {
        if output.status.success() {
            println!(
                "✓ {} - {} is already mounted at {}",
                server_name, share_name, mount_point
            );
            return Ok(());
        }
    }

    // Get user ID and group ID using native Rust
    #[cfg(unix)]
    let (uid, gid) = {
        let uid_val = exec.get_uid()?;
        let gid_val = exec.get_gid()?;
        (uid_val.to_string(), gid_val.to_string())
    };
    #[cfg(not(unix))]
    let (uid, gid) = {
        // Fallback to commands on non-Unix
        let uid_output = exec.execute_simple("id", &["-u"])?;
        let uid = String::from_utf8_lossy(&uid_output.stdout)
            .trim()
            .to_string();
        let gid_output = exec.execute_simple("id", &["-g"])?;
        let gid = String::from_utf8_lossy(&gid_output.stdout)
            .trim()
            .to_string();
        (uid, gid)
    };

    // Create credentials file for secure password handling
    // This avoids password exposure in command line and handles special characters
    // Must be created with sudo since mount runs as root
    let creds_file = format!("/tmp/.smb_creds_{}_{}", server_name, share_name);
    let creds_content = format!("username={}\npassword={}\n", username, password);
    
    // Write credentials file to temp location first, then move with sudo (interactive for password prompt)
    let temp_creds = format!("/tmp/smb_creds_temp_{}", std::process::id());
    exec.write_file(&temp_creds, creds_content.as_bytes())?;
    exec.execute_interactive("sudo", &["mv", &temp_creds, &creds_file])?;
    exec.execute_interactive("sudo", &["chmod", "600", &creds_file])?;

    // Build mount options using credentials file
    let mut mount_opts = format!(
        "credentials={},uid={},gid={}",
        creds_file, uid, gid
    );
    if let Some(ref opts) = server_config.options {
        mount_opts.push_str(&format!(",{}", opts));
    }

    println!("Mounting: {} -> {}", share_path, mount_point);

    // Mount the share
    let mount_result = exec.execute_simple(
        "sudo",
        &[
            "mount",
            "-t",
            "cifs",
            share_path,
            mount_point,
            "-o",
            &mount_opts,
        ],
    );
    
    // Clean up credentials file after mount attempt (use sudo since we created it with sudo)
    let _ = exec.execute_interactive("sudo", &["rm", "-f", &creds_file]);

    if mount_result.is_ok() && mount_result.as_ref().unwrap().status.success() {
        println!(
            "✓ {} - {} mounted at {}",
            server_name, share_name, mount_point
        );

        // For fstab, we need to use a persistent credentials file
        // Use a location that persists: /etc/smb-credentials/
        let persistent_creds_dir = "/etc/smb-credentials";
        let persistent_creds_file = format!("{}/{}_{}", persistent_creds_dir, server_name, share_name);
        
        // Create credentials directory if it doesn't exist (interactive for password prompt)
        exec.execute_interactive("sudo", &["mkdir", "-p", persistent_creds_dir])?;
        
        // Write persistent credentials file
        let creds_content = format!("username={}\npassword={}\n", username, password);
        exec.write_file("/tmp/smb_creds_persist", creds_content.as_bytes())?;
        exec.execute_simple("sudo", &["mv", "/tmp/smb_creds_persist", &persistent_creds_file])?;
        exec.execute_simple("sudo", &["chmod", "600", &persistent_creds_file])?;
        
        // Build fstab mount options with persistent credentials file
        let fstab_mount_opts = format!(
            "credentials={},uid={},gid={}",
            persistent_creds_file, uid, gid
        );
        let fstab_opts = if let Some(ref opts) = server_config.options {
            format!("{},{}", fstab_mount_opts, opts)
        } else {
            fstab_mount_opts
        };
        let fstab_entry = format!(
            "{} {} cifs {},_netdev 0 0",
            share_path, mount_point, fstab_opts
        );
        add_fstab_entry(exec, mount_point, &fstab_entry)?;
    } else {
        anyhow::bail!(
            "Failed to mount {} - {} at {}",
            server_name,
            share_name,
            mount_point
        );
    }

    Ok(())
}

fn add_fstab_entry<E: CommandExecutor>(exec: &E, mount_point: &str, entry: &str) -> Result<()> {
    // Check if entry already exists
    let fstab_content = exec.read_file("/etc/fstab")?;
    if fstab_content.lines().any(|line| line.contains(mount_point)) {
        println!("✓ Entry already exists in /etc/fstab");
        return Ok(());
    }

    // Append entry to /etc/fstab
    let new_content = format!("{}\n{}", fstab_content.trim_end(), entry);
    exec.write_file("/tmp/fstab.new", new_content.as_bytes())?;
    exec.execute_interactive("sudo", &["mv", "/tmp/fstab.new", "/etc/fstab"])?;
    println!("✓ Added to /etc/fstab for automatic mounting");
    println!("  Entry: {}", entry);
    Ok(())
}

fn remove_fstab_entry<E: CommandExecutor>(exec: &E, mount_point: &str) -> Result<()> {
    let fstab_content = exec.read_file("/etc/fstab")?;
    let filtered_lines: Vec<&str> = fstab_content
        .lines()
        .filter(|line| !line.contains(mount_point))
        .collect();

    if filtered_lines.len() == fstab_content.lines().count() {
        // No entry found, nothing to remove
        return Ok(());
    }

    let new_content = filtered_lines.join("\n");
    if !new_content.is_empty() {
        exec.write_file("/tmp/fstab.new", new_content.as_bytes())?;
        exec.execute_interactive("sudo", &["mv", "/tmp/fstab.new", "/etc/fstab"])?;
    }
    Ok(())
}

fn uninstall_smb_mounts_remote<E: CommandExecutor>(exec: &E, config: &EnvConfig) -> Result<()> {
    println!("=== Unmounting SMB shares ===");

    // Unmount each share
    for (server_name, server_config) in &config.smb_servers {
        for share_name in &server_config.shares {
            let mount_point = format!("/mnt/smb/{}/{}", server_name, share_name);

            // Check if mounted
            let mountpoint_check = exec.execute_simple("mountpoint", &["-q", &mount_point]);
            if let Ok(output) = mountpoint_check {
                if output.status.success() {
                    println!("Unmounting {} - {}...", server_name, share_name);
                    let umount_result = exec.execute_simple("sudo", &["umount", &mount_point]);
                    if umount_result.is_ok() && umount_result.as_ref().unwrap().status.success() {
                        println!("✓ {} - {} unmounted", server_name, share_name);
                    } else {
                        println!("✗ Failed to unmount {} - {}", server_name, share_name);
                    }
                } else {
                    println!("✓ {} - {} is not mounted", server_name, share_name);
                }
            }

            // Remove from /etc/fstab
            remove_fstab_entry(exec, &mount_point)?;
            println!("✓ Removed {} from /etc/fstab", mount_point);

            // Remove mount point directory using native Rust check
            if exec.is_directory(&mount_point)? {
                let rmdir_result = exec.execute_simple("sudo", &["rmdir", &mount_point]);
                if rmdir_result.is_ok() && rmdir_result.as_ref().unwrap().status.success() {
                    println!("✓ Removed mount point {}", mount_point);
                } else {
                    println!("Mount point {} not empty, leaving it", mount_point);
                }
            }
        }
    }

    println!();
    println!("=== SMB uninstall complete ===");

    Ok(())
}

// Removed build_smb_uninstall_script - replaced with uninstall_smb_mounts_remote
// Removed execute_smb_script - replaced with direct SshConnection usage
