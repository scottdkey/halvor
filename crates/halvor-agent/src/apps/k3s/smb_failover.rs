//! SMB failover setup for k3s data directory
//! 
//! Sets up automatic failover between maple (primary) and willow (fallback) SMB servers
//! for the k3s data directory. Uses a unified path that automatically switches based on availability.

use halvor_core::utils::exec::CommandExecutor;
use anyhow::Result;

/// Set up SMB failover for k3s data directory
///
/// Creates a systemd service that:
/// 1. Checks maple availability
/// 2. Creates symlink from unified path to active server (maple or willow)
/// 3. Runs before k3s starts to ensure data directory is available
#[allow(dead_code)]
pub fn setup_smb_failover<E: CommandExecutor>(exec: &E, hostname: &str) -> Result<()> {
    println!("Setting up SMB failover for k3s data directory...");
    
    // Create the failover script
    let script_path = "/usr/local/bin/k3s-smb-failover.sh";
    let script_content = format!(
        r#"#!/bin/bash
# K3s SMB Failover Script
# Automatically switches between maple (primary) and willow (fallback) SMB servers

set -euo pipefail

HOSTNAME="{}"
UNIFIED_DIR="/mnt/smb/halvor/k3s/${{HOSTNAME}}"
MAPLE_DIR="/mnt/smb/maple/halvor/k3s/${{HOSTNAME}}"
WILLOW_DIR="/mnt/smb/willow/halvor/k3s/${{HOSTNAME}}"

# Function to check if a mount point is available
check_mount() {{
    local mount_point="$1"
    if mountpoint -q "$mount_point" 2>/dev/null; then
        # Check if directory is writable
        if [ -w "$mount_point" ]; then
            return 0
        fi
    fi
    return 1
}}

# Create parent directories
mkdir -p "$(dirname "$MAPLE_DIR")"
mkdir -p "$(dirname "$WILLOW_DIR")"
mkdir -p "$(dirname "$UNIFIED_DIR")"

# Remove existing symlink if it exists
if [ -L "$UNIFIED_DIR" ]; then
    rm -f "$UNIFIED_DIR"
fi

# Try maple first (primary)
if check_mount "/mnt/smb/maple/halvor"; then
    echo "Using maple (primary) SMB server"
    mkdir -p "$MAPLE_DIR"
    ln -sfn "$MAPLE_DIR" "$UNIFIED_DIR"
    echo "✓ Symlink created: $UNIFIED_DIR -> $MAPLE_DIR"
    exit 0
fi

# Fallback to willow
if check_mount "/mnt/smb/willow/halvor"; then
    echo "Using willow (fallback) SMB server"
    mkdir -p "$WILLOW_DIR"
    ln -sfn "$WILLOW_DIR" "$UNIFIED_DIR"
    echo "✓ Symlink created: $UNIFIED_DIR -> $WILLOW_DIR"
    exit 0
fi

# Neither server is available - create local fallback directory
echo "WARNING: Neither maple nor willow SMB servers are available!"
echo "  Check SMB mounts:"
echo "    - /mnt/smb/maple/halvor"
echo "    - /mnt/smb/willow/halvor"
echo "  Creating local fallback directory..."
LOCAL_FALLBACK="/var/lib/rancher/k3s/data-fallback"
mkdir -p "$LOCAL_FALLBACK"
mkdir -p "$(dirname "$UNIFIED_DIR")"
ln -sfn "$LOCAL_FALLBACK" "$UNIFIED_DIR"
echo "✓ Created local fallback symlink: $UNIFIED_DIR -> $LOCAL_FALLBACK"
echo "  Note: Data will be stored locally until SMB mounts are available"
exit 0
"#,
        hostname
    );
    
    // Write script to temp file first, then move with sudo
    exec.write_file("/tmp/k3s-smb-failover.sh", script_content.as_bytes())?;
    exec.execute_shell(&format!(
        "sudo mv /tmp/k3s-smb-failover.sh {} && sudo chmod +x {}",
        script_path, script_path
    ))?;
    println!("✓ Created failover script: {}", script_path);
    
    // Create systemd service
    let service_content = format!(
        r#"[Unit]
Description=K3s SMB Failover - Setup data directory with automatic failover
Before=k3s.service
After=network-online.target
Wants=network-online.target

[Service]
Type=oneshot
ExecStart={}
RemainAfterExit=yes
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=k3s.service
"#,
        script_path
    );
    
    let service_file = "/etc/systemd/system/k3s-smb-failover.service";
    exec.write_file("/tmp/k3s-smb-failover.service", service_content.as_bytes())?;
    exec.execute_shell(&format!(
        "sudo mv /tmp/k3s-smb-failover.service {} && sudo systemctl daemon-reload",
        service_file
    ))?;
    println!("✓ Created systemd service: {}", service_file);
    
    // Enable the service
    exec.execute_shell("sudo systemctl enable k3s-smb-failover.service")?;
    println!("✓ Enabled k3s-smb-failover service");
    
    // Run the script once to set up the symlink immediately
    println!("Running failover script to set up initial symlink...");
    let result = exec.execute_shell(&format!("sudo {}", script_path));
    if let Ok(output) = result {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            if !output_str.trim().is_empty() {
                print!("{}", output_str);
            }
            println!("✓ Failover symlink configured");
        } else {
            let error_str = String::from_utf8_lossy(&output.stderr);
            println!("⚠️  Warning: Failover script had issues: {}", error_str);
            println!("   This may be normal if SMB mounts are not yet available.");
            println!("   The service will retry when k3s starts.");
        }
    }
    
    // Update k3s service to depend on the failover service
    // Use Wants instead of Requires so k3s can start even if SMB mounts aren't available
    let service_override_dir = "/etc/systemd/system/k3s.service.d";
    let override_content = r#"[Unit]
After=k3s-smb-failover.service
Wants=k3s-smb-failover.service
"#;
    
    exec.execute_shell(&format!("sudo mkdir -p {}", service_override_dir))?;
    let override_file = format!("{}/20-smb-failover.conf", service_override_dir);
    exec.write_file("/tmp/k3s-smb-override.conf", override_content.as_bytes())?;
    exec.execute_shell(&format!(
        "sudo mv /tmp/k3s-smb-override.conf {} && sudo systemctl daemon-reload",
        override_file
    ))?;
    println!("✓ Updated k3s service to depend on SMB failover");
    
    println!("✓ SMB failover setup complete");
    println!();
    println!("  Data directory: /mnt/smb/halvor/k3s/{}", hostname);
    println!("  Primary: /mnt/smb/maple/halvor/k3s/{}", hostname);
    println!("  Fallback: /mnt/smb/willow/halvor/k3s/{}", hostname);
    println!();
    
    Ok(())
}

