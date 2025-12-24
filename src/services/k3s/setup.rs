//! Complete K3s cluster setup
//!
//! Handles full cluster setup including:
//! - Node provisioning (SSH keys, sudo, Tailscale)
//! - SMB mount configuration on deployment nodes
//! - Primary control plane initialization
//! - Additional control plane node joining
//! - Cluster verification

use crate::config::EnvConfig;
use crate::services::k3s::{init_control_plane, join_cluster, verify_ha_cluster};
use crate::services::smb;
use crate::services::tailscale;
use crate::utils::exec::{CommandExecutor, Executor};
use crate::utils::ssh;
use anyhow::{Context, Result};

/// Get target host (prefer Tailscale hostname, fallback to IP) from config
fn get_target_host(hostname: &str, config: &EnvConfig) -> Result<String> {
    let actual_hostname = crate::config::service::find_hostname_in_config(hostname, config)
        .ok_or_else(|| anyhow::anyhow!("Host '{}' not found in config", hostname))?;
    let host_config = config
        .hosts
        .get(&actual_hostname)
        .with_context(|| format!("Host '{}' not found in config", hostname))?;

    // Prefer Tailscale hostname over IP for SSH connections
    if let Some(hostname_val) = &host_config.hostname {
        // First, try to get actual Tailscale hostname from local Tailscale status
        use crate::services::tailscale::get_peer_tailscale_hostname;
        if let Ok(Some(ts_hostname)) = get_peer_tailscale_hostname(hostname_val) {
            // Found actual Tailscale hostname - use it
            return Ok(ts_hostname);
        } else if hostname_val.contains('.') {
            // Hostname already includes domain - use as-is
            return Ok(hostname_val.clone());
        } else {
            // Construct from tailnet base
            return Ok(format!("{}.{}", hostname_val, config._tailnet_base));
        }
    }

    // Fallback to IP if no hostname configured
    if let Some(ip) = &host_config.ip {
        Ok(ip.clone())
    } else {
        anyhow::bail!("No IP or Tailscale hostname configured for {}", hostname);
    }
}

/// Check sudo access on remote host
fn check_sudo_access<E: CommandExecutor>(exec: &E, is_remote: bool) -> Result<()> {
    if !exec.is_linux()? {
        println!("✓ macOS detected (Docker Desktop handles permissions)");
        return Ok(());
    }

    if is_remote {
        println!("Verifying sudo access (you may be prompted for password)...");
        exec.execute_interactive("sudo", &["sh", "-c", "true"])
            .context("Failed to verify sudo access. Please ensure you have sudo privileges.")?;
        println!("✓ Sudo access verified");
    } else {
        println!("Verifying sudo access (you may be prompted for password)...");
        exec.execute_interactive("sudo", &["sh", "-c", "true"])
            .context("Failed to verify sudo access. Please ensure you have sudo privileges.")?;
        println!("✓ Sudo access verified");
    }
    Ok(())
}

/// Check if SSH key authentication already works over Tailscale
fn check_ssh_key_auth(hostname: &str, config: &EnvConfig) -> Result<bool> {
    let exec = Executor::new(hostname, config)?;
    if exec.is_local() {
        return Ok(true); // Localhost doesn't need SSH
    }

    // Get the target host (should be Tailscale hostname)
    let target_host = get_target_host(hostname, config)?;

    // Try with default username first (most common case)
    let default_username = crate::config::get_default_username();
    let host_str = format!("{}@{}", default_username, target_host);

    // Use the same test as SshConnection::new to check if key-based auth works
    use std::process::{Command, Stdio};
    let test_output = Command::new("ssh")
        .args([
            "-o",
            "ConnectTimeout=10",
            "-o",
            "BatchMode=yes",
            "-o",
            "PreferredAuthentications=publickey",
            "-o",
            "PasswordAuthentication=no",
            "-o",
            "StrictHostKeyChecking=no",
            &host_str,
            "echo",
            "test",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output();

    // If default username works, we're done
    if test_output.is_ok() && test_output.unwrap().status.success() {
        return Ok(true);
    }

    // If default username failed, try to get username from Executor
    // (which handles SSH config lookup)
    // For now, if default username doesn't work, assume SSH isn't configured
    // The user can use --skip-ssh if they've configured SSH differently
    Ok(false)
}

/// Provision a single node (SSH keys, sudo, Tailscale, SMB)
fn provision_node(
    hostname: &str,
    config: &EnvConfig,
    skip_smb: bool,
    skip_ssh: bool,
) -> Result<()> {
    let exec = Executor::new(hostname, config)?;
    let is_local = exec.is_local();

    // Step 1: Setup SSH keys (skip for localhost or if already configured)
    if !is_local && !skip_ssh {
        // Check if SSH key auth already works
        match check_ssh_key_auth(hostname, config) {
            Ok(true) => {
                println!("✓ SSH key authentication already configured");
            }
            Ok(false) | Err(_) => {
                println!("Setting up SSH keys...");
                let target_host = get_target_host(hostname, config)?;
                match ssh::copy_ssh_key(&target_host, None, None) {
                    Ok(_) => {
                        println!("✓ SSH keys configured");
                    }
                    Err(e) => {
                        eprintln!("⚠️  Warning: Failed to setup SSH keys: {}", e);
                        eprintln!(
                            "   Continuing anyway - ensure SSH access is configured manually"
                        );
                        eprintln!("   You can skip SSH setup with --skip-ssh flag");
                    }
                }
            }
        }
    } else if skip_ssh {
        println!("Skipping SSH key setup (--skip-ssh)");
    }

    // Step 2: Verify sudo access
    check_sudo_access(&exec, !is_local)
        .with_context(|| format!("Failed to verify sudo access on {}", hostname))?;

    // Step 3: Install and verify Tailscale
    println!("Installing Tailscale (required for cluster communication)...");
    tailscale::check_and_install_remote(&exec)
        .with_context(|| format!("Failed to install Tailscale on {}", hostname))?;

    // Verify Tailscale is running
    let tailscale_ip = tailscale::get_tailscale_ip_with_fallback(&exec, hostname, config)
        .with_context(|| format!("Failed to get Tailscale IP for {}", hostname))?;
    println!("✓ Tailscale is running with IP: {}", tailscale_ip);

    // Step 4: Setup SMB mounts (if not skipped)
    if !skip_smb && !config.smb_servers.is_empty() {
        println!("Setting up SMB mounts...");
        smb::setup_smb_mounts(hostname, config)
            .with_context(|| format!("Failed to setup SMB mounts on {}", hostname))?;
        println!("✓ SMB mounts configured");
    } else if skip_smb {
        println!("Skipping SMB mount setup (--skip-smb)");
    } else {
        println!("No SMB servers configured - skipping SMB mount setup");
    }

    Ok(())
}

/// Complete cluster setup
pub fn setup_cluster(
    primary: &str,
    additional_nodes: &[&str],
    smb_nodes: Option<&[&str]>,
    skip_smb: bool,
    skip_ssh: bool,
    skip_verify: bool,
    config: &EnvConfig,
) -> Result<()> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("K3s HA Cluster Setup");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Primary control plane: {}", primary);
    println!(
        "Additional control plane nodes: {}",
        additional_nodes.join(", ")
    );
    println!();

    // Collect all nodes that need provisioning
    let all_nodes: Vec<&str> = std::iter::once(primary)
        .chain(additional_nodes.iter().copied())
        .collect();

    // Step 1: Provision all nodes (SSH keys, sudo, Tailscale, SMB)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Step 1: Provisioning nodes");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    for (idx, node) in all_nodes.iter().enumerate() {
        println!(
            "Provisioning node {}/{}: {}",
            idx + 1,
            all_nodes.len(),
            node
        );
        println!();

        // Determine if this node needs SMB setup
        let node_skip_smb = if let Some(smb_list) = smb_nodes {
            !smb_list.contains(node)
        } else {
            skip_smb
        };

        provision_node(node, config, node_skip_smb, skip_ssh)
            .with_context(|| format!("Failed to provision {}", node))?;

        println!("✓ {} provisioned successfully", node);
        println!();
    }

    // Step 2: Initialize primary control plane
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Step 2: Initializing primary control plane");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("Initializing {} as primary control plane...", primary);
    init_control_plane(primary, None, true, config)
        .with_context(|| format!("Failed to initialize primary control plane on {}", primary))?;
    println!("✓ Primary control plane initialized");
    println!();

    // Step 3: Get cluster join information from primary
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Step 3: Getting cluster join information");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    use crate::services::k3s::status::get_cluster_join_info;
    let (server_addr, cluster_token) = get_cluster_join_info(primary, config)
        .with_context(|| format!("Failed to get cluster join info from {}", primary))?;

    println!("✓ Cluster server address: {}", server_addr);
    println!("✓ Cluster token retrieved");
    println!();

    // Step 4: Join additional control plane nodes
    if !additional_nodes.is_empty() {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Step 4: Joining additional control plane nodes");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!();

        for (idx, node) in additional_nodes.iter().enumerate() {
            println!(
                "Joining node {}/{}: {}",
                idx + 1,
                additional_nodes.len(),
                node
            );
            join_cluster(node, &server_addr, &cluster_token, true, config)
                .with_context(|| format!("Failed to join {} to cluster", node))?;
            println!("✓ {} joined as control plane node", node);
            println!();
        }
    }

    // Step 5: Verify cluster
    if !skip_verify {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Step 5: Verifying cluster health");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!();

        let all_nodes: Vec<&str> = std::iter::once(primary)
            .chain(additional_nodes.iter().copied())
            .collect();

        verify_ha_cluster(primary, &all_nodes, config).context("Cluster verification failed")?;
        println!();
    } else {
        println!("Skipping cluster verification (--skip-verify)");
        println!();
    }

    // Summary
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✓ Cluster setup complete!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Cluster nodes:");
    println!("  • Primary: {}", primary);
    for node in additional_nodes {
        println!("  • Control plane: {}", node);
    }
    println!();
    println!("Next steps:");
    println!(
        "  • Check cluster status:    halvor k3s status -H {}",
        primary
    );
    println!(
        "  • Get kubeconfig:          halvor k3s kubeconfig -H {} --merge",
        primary
    );
    println!("  • Deploy SMB storage:      halvor install smb-storage --helm");
    println!("  • Deploy services:        halvor install <service> --helm");
    println!();

    Ok(())
}
