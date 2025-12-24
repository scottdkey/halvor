//! Provision Service
//!
//! Sets up Linux machines for Kubernetes cluster membership.
//! Handles SSH keys, sudo access, Tailscale networking, and K3s installation.

use crate::config::EnvConfig;
use crate::services::k3s;
use crate::services::tailscale;
use crate::utils::exec::{CommandExecutor, Executor, PackageManager};
use crate::utils::ssh;
use anyhow::{Context, Result};
use std::io::{self, BufRead, Write};

/// Automatically provision multiple nodes to join an existing cluster
pub fn provision_cluster_nodes(
    primary_hostname: &str,
    node_hostnames: &[&str],
    config: &EnvConfig,
) -> Result<()> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Automated K3s Cluster Setup");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Primary node: {}", primary_hostname);
    println!("Nodes to join: {}", node_hostnames.join(", "));
    println!();

    // Get cluster join information from primary node
    println!(
        "Getting cluster join information from {}...",
        primary_hostname
    );
    let (server_addr, token) = k3s::get_cluster_join_info(primary_hostname, config)
        .context("Failed to get cluster join information from primary node")?;

    println!("✓ Server address: {}", server_addr);
    println!("✓ Cluster token retrieved");
    println!();

    // Provision each node
    for (idx, node) in node_hostnames.iter().enumerate() {
        println!();
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!(
            "Provisioning node {}/{}: {}",
            idx + 1,
            node_hostnames.len(),
            node
        );
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!();

        // Use non-interactive provisioning with the cluster info
        provision_defaults(
            node,
            ClusterRole::JoinControlPlane,
            Some(&server_addr),
            Some(&token),
            config,
        )?;

        println!();
        println!("✓ {} provisioned successfully", node);
    }

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("All nodes provisioned successfully!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Verifying cluster health...");
    println!();

    // Verify the cluster
    let all_nodes: Vec<&str> = std::iter::once(primary_hostname)
        .chain(node_hostnames.iter().copied())
        .collect();
    k3s::verify_ha_cluster(primary_hostname, &all_nodes, config)?;

    Ok(())
}

/// Cluster role for K3s node
#[derive(Debug, Clone, PartialEq)]
pub enum ClusterRole {
    /// First control plane node - initializes the cluster
    InitControlPlane,
    /// Additional control plane node - joins existing cluster as control plane
    JoinControlPlane,
    /// Worker/agent node - joins cluster as worker only
    JoinAgent,
}

/// Options for provisioning a node
#[derive(Debug, Clone)]
pub struct ProvisionOptions {
    /// The hostname or IP of the target machine
    pub hostname: String,
    /// SSH user for initial connection (before key setup)
    pub ssh_user: Option<String>,
    /// Role this node will play in the cluster
    pub cluster_role: ClusterRole,
    /// For join operations: the control plane server to join
    pub cluster_server: Option<String>,
    /// Cluster join token (generated for init, required for join)
    pub cluster_token: Option<String>,
    /// Skip confirmation prompts
    pub skip_prompts: bool,
}

/// Get target host (IP or hostname) from config
fn get_target_host(hostname: &str, config: &EnvConfig) -> Result<String> {
    let actual_hostname = crate::config::service::find_hostname_in_config(hostname, config)
        .ok_or_else(|| anyhow::anyhow!("Host '{}' not found in config", hostname))?;
    let host_config = config
        .hosts
        .get(&actual_hostname)
        .with_context(|| format!("Host '{}' not found in config", hostname))?;

    if let Some(ip) = &host_config.ip {
        Ok(ip.clone())
    } else if let Some(hostname) = &host_config.hostname {
        Ok(hostname.clone())
    } else {
        anyhow::bail!("No IP or hostname configured for {}", hostname);
    }
}

/// Provision with defaults (non-interactive)
/// Sets up a Linux machine for Kubernetes cluster membership
pub fn provision_defaults(
    hostname: &str,
    cluster_role: ClusterRole,
    cluster_server: Option<&str>,
    cluster_token: Option<&str>,
    config: &EnvConfig,
) -> Result<()> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Provisioning {} for Kubernetes cluster", hostname);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    let exec = Executor::new(hostname, config)?;
    let is_local = exec.is_local();

    // Step 1: Setup SSH keys (skip for localhost)
    if !is_local {
        println!("\n=== Step 1: Setting up SSH keys ===");
        let target_host = get_target_host(hostname, config)?;
        ssh::copy_ssh_key(&target_host, None, None)?;
    } else {
        println!("\n=== Step 1: SSH keys (skipped for localhost) ===");
    }

    // Step 2: Verify sudo access (will prompt for password if needed)
    println!("\n=== Step 2: Verifying sudo access ===");
    check_sudo_access(&exec, !is_local)?;

    // Step 3: Install and verify Tailscale (required for cluster communication)
    println!("\n=== Step 3: Installing Tailscale (required) ===");
    println!("Tailscale is required for cluster communication between nodes.");
    tailscale::check_and_install_remote(&exec)?;

    // Verify Tailscale is running and get IP
    println!("Verifying Tailscale is running...");
    let tailscale_ip = tailscale::get_tailscale_ip_with_fallback(&exec, hostname, config)?;
    println!("✓ Tailscale is running with IP: {}", tailscale_ip);

    // Step 4: Setup SMB mounts (for data persistence)
    println!("\n=== Step 4: Setting up SMB mounts ===");
    if !config.smb_servers.is_empty() {
        println!("Setting up SMB mounts for persistent storage...");
        crate::services::smb::setup_smb_mounts(hostname, config)?;
    } else {
        println!("No SMB servers configured - skipping SMB mount setup");
    }

    // Step 5: Install and configure K3s based on role
    println!("\n=== Step 5: Installing K3s ===");
    match cluster_role {
        ClusterRole::InitControlPlane => {
            k3s::init_control_plane(hostname, cluster_token, true, config)?;
        }
        ClusterRole::JoinControlPlane => {
            let server = cluster_server.ok_or_else(|| {
                anyhow::anyhow!("--cluster-server is required when joining as control plane")
            })?;
            let token = cluster_token.ok_or_else(|| {
                anyhow::anyhow!("--cluster-token is required when joining cluster")
            })?;
            k3s::join_cluster(hostname, server, token, true, config)?;
        }
        ClusterRole::JoinAgent => {
            let server = cluster_server.ok_or_else(|| {
                anyhow::anyhow!("--cluster-server is required when joining as agent")
            })?;
            let token = cluster_token.ok_or_else(|| {
                anyhow::anyhow!("--cluster-token is required when joining cluster")
            })?;
            k3s::join_cluster(hostname, server, token, false, config)?;
        }
    }

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✓ Provisioning complete for {}", hostname);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}

/// Guided interactive provisioning
pub fn provision_guided(hostname: &str, config: &EnvConfig) -> Result<()> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Halvor Guided Provisioning - Kubernetes Cluster Setup");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!(
        "This wizard will help you set up {} for Kubernetes cluster membership.",
        hostname
    );
    println!();

    let exec = Executor::new(hostname, config)?;
    let is_local = exec.is_local();

    if is_local {
        println!("Target: localhost (local execution)");
    } else {
        println!("Target: {} (remote execution via SSH)", hostname);
    }
    println!();

    // Step 1: Setup SSH keys (skip for localhost)
    if !is_local {
        println!();
        println!("┌─────────────────────────────────────────────────────────────────────────┐");
        println!("│ Step 1: SSH Keys                                                        │");
        println!("└─────────────────────────────────────────────────────────────────────────┘");
        println!();
        if prompt_yn("Set up SSH key authentication?", true)? {
            // Get target host from config for SSH key copying
            let host_config = config
                .hosts
                .get(
                    &crate::config::service::find_hostname_in_config(hostname, config).ok_or_else(
                        || anyhow::anyhow!("Host '{}' not found in config", hostname),
                    )?,
                )
                .with_context(|| format!("Host '{}' not found in config", hostname))?;

            let target_host = if let Some(ip) = &host_config.ip {
                ip.as_str()
            } else if let Some(hostname) = &host_config.hostname {
                hostname.as_str()
            } else {
                anyhow::bail!("No IP or hostname configured for {}", hostname);
            };

            crate::utils::ssh::copy_ssh_key(target_host, None, None)?;
        }
    }

    // Step 2: Verify sudo access (will prompt for password if needed)
    println!();
    println!("┌─────────────────────────────────────────────────────────────────────────┐");
    println!("│ Step 2: Sudo Access                                                     │");
    println!("└─────────────────────────────────────────────────────────────────────────┘");
    println!();
    check_sudo_access(&exec, !is_local)?;

    // Step 3: Tailscale (required)
    println!();
    println!("┌─────────────────────────────────────────────────────────────────────────┐");
    println!("│ Step 3: Tailscale (required)                                             │");
    println!("└─────────────────────────────────────────────────────────────────────────┘");
    println!();
    println!("Tailscale is required for cluster communication between nodes.");
    println!("The cluster will use Tailscale IPs for all node-to-node communication.");

    if tailscale::is_tailscale_installed(&exec) {
        println!("✓ Tailscale is already installed");
    } else {
        println!("Installing Tailscale...");
        tailscale::check_and_install_remote(&exec)?;
    }

    // Verify Tailscale is running and get IP
    println!("Verifying Tailscale is running...");
    let tailscale_ip = tailscale::get_tailscale_ip_with_fallback(&exec, hostname, config)?;
    println!("✓ Tailscale is running with IP: {}", tailscale_ip);

    // Step 4: Kubernetes Cluster Role
    println!();
    println!("┌─────────────────────────────────────────────────────────────────────────┐");
    println!("│ Step 4: Kubernetes Cluster Role                                         │");
    println!("└─────────────────────────────────────────────────────────────────────────┘");
    println!();
    println!("Select the role this node will play in the cluster:");
    println!();
    println!("  Init Control Plane:  First node - initializes a new HA cluster");
    println!("  Join Control Plane:  Additional control plane node (for HA)");
    println!("  Join Agent:          Worker node (joins as agent only)");
    println!();

    let role_choice = prompt_choice(
        "What role should this node have?",
        &["Init Control Plane", "Join Control Plane", "Join Agent"],
        0,
    )?;

    let cluster_role = match role_choice {
        0 => ClusterRole::InitControlPlane,
        1 => ClusterRole::JoinControlPlane,
        2 => ClusterRole::JoinAgent,
        _ => unreachable!(),
    };

    let (cluster_server, cluster_token) = match cluster_role {
        ClusterRole::InitControlPlane => {
            // Ensure openssl is installed for token generation
            ensure_openssl_installed(&exec)?;

            // Generate token using k3s service function
            println!("Generating cluster token...");
            let token =
                k3s::generate_cluster_token().context("Failed to generate cluster token")?;
            println!();
            println!("Generated cluster token: {}", token);
            println!("Save this token to join additional nodes!");
            (None, Some(token))
        }
        ClusterRole::JoinControlPlane | ClusterRole::JoinAgent => {
            // Try to get token from environment first (from 1Password)
            let token = std::env::var("K3S_TOKEN")
                .ok()
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty());

            let (server, token) = if let Some(token_val) = token {
                println!("Using K3S_TOKEN from environment");
                // Try to get server from environment or prompt
                let server = std::env::var("K3S_SERVER")
                    .ok()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());

                if let Some(server_val) = server {
                    println!("Using K3S_SERVER from environment: {}", server_val);
                    (Some(server_val), Some(token_val))
                } else {
                    print!("Enter the control plane server address (hostname or IP): ");
                    io::stdout().flush()?;
                    let mut server_input = String::new();
                    io::stdin().lock().read_line(&mut server_input)?;
                    let server_input = server_input.trim().to_string();
                    if server_input.is_empty() {
                        anyhow::bail!("Server address is required");
                    }
                    (Some(server_input), Some(token_val))
                }
            } else {
                // No token in environment, prompt for both
                print!("Enter the control plane server address (hostname or IP): ");
                io::stdout().flush()?;
                let mut server = String::new();
                io::stdin().lock().read_line(&mut server)?;
                let server = server.trim().to_string();
                if server.is_empty() {
                    anyhow::bail!("Server address is required");
                }

                print!("Enter the cluster join token: ");
                io::stdout().flush()?;
                let mut token = String::new();
                io::stdin().lock().read_line(&mut token)?;
                let token = token.trim().to_string();
                if token.is_empty() {
                    anyhow::bail!("Cluster token is required");
                }

                (Some(server), Some(token))
            };

            (server, token)
        }
    };

    // Step 5: Install K3s
    println!();
    println!("┌─────────────────────────────────────────────────────────────────────────┐");
    println!("│ Step 5: Installing K3s                                                  │");
    println!("└─────────────────────────────────────────────────────────────────────────┘");
    println!();

    match cluster_role {
        ClusterRole::InitControlPlane => {
            // Verify token is present and not empty before passing to k3s
            let token_to_pass = cluster_token
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("Cluster token is missing"))?;
            if token_to_pass.is_empty() {
                anyhow::bail!("Cluster token is empty. Token generation may have failed.");
            }
            k3s::init_control_plane(hostname, Some(token_to_pass), false, config)?;
        }
        ClusterRole::JoinControlPlane => {
            k3s::join_cluster(
                hostname,
                cluster_server.as_ref().unwrap(),
                cluster_token.as_ref().unwrap(),
                true,
                config,
            )?;
        }
        ClusterRole::JoinAgent => {
            k3s::join_cluster(
                hostname,
                cluster_server.as_ref().unwrap(),
                cluster_token.as_ref().unwrap(),
                false,
                config,
            )?;
        }
    }

    // Summary
    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✓ Provisioning complete for {}", hostname);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Next steps:");
    println!(
        "  • Check cluster status:    halvor k3s status -H {}",
        hostname
    );
    println!(
        "  • Get kubeconfig:          halvor k3s kubeconfig -H {}",
        hostname
    );
    println!("  • Install services:        halvor helm install <chart>");
    println!("  • List available charts:   halvor helm charts");

    Ok(())
}

/// Check sudo access (works for both local and remote)
/// Prompts for password if needed (no passwordless sudo required)
pub fn check_sudo_access<E: CommandExecutor>(exec: &E, _is_remote: bool) -> Result<()> {
    println!("=== Checking sudo access ===");

    if !exec.is_linux()? {
        println!("✓ macOS detected (Docker Desktop handles permissions)");
        return Ok(());
    }

    // Use interactive mode to prompt for password if needed (works for both local and remote)
    println!("Testing sudo access (you may be prompted for your password)...");
    exec.execute_interactive("sudo", &["sh", "-c", "true"])?;
    println!("✓ Sudo access verified");

    Ok(())
}

/// Prompt for yes/no with default
fn prompt_yn(question: &str, default: bool) -> Result<bool> {
    let default_hint = if default { "[Y/n]" } else { "[y/N]" };
    print!("{} {} ", question, default_hint);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().lock().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    if input.is_empty() {
        return Ok(default);
    }

    Ok(input.starts_with('y'))
}

/// Ensure openssl is installed on the remote host
fn ensure_openssl_installed<E: CommandExecutor>(exec: &E) -> Result<()> {
    // Check if openssl is already installed
    if exec.check_command_exists("openssl")? {
        println!("✓ openssl is already installed");
        return Ok(());
    }

    println!("openssl not found. Installing openssl...");

    // Detect package manager
    let pkg_mgr = PackageManager::detect(exec)?;

    match pkg_mgr {
        PackageManager::Apt => {
            println!("Installing openssl via apt...");
            pkg_mgr.install_package(exec, "openssl")?;
        }
        PackageManager::Yum => {
            println!("Installing openssl via yum...");
            pkg_mgr.install_package(exec, "openssl")?;
        }
        PackageManager::Dnf => {
            println!("Installing openssl via dnf...");
            pkg_mgr.install_package(exec, "openssl")?;
        }
        PackageManager::Brew => {
            println!("Installing openssl via brew...");
            pkg_mgr.install_package(exec, "openssl")?;
        }
        PackageManager::Unknown => {
            anyhow::bail!("No supported package manager found. Please install openssl manually.");
        }
    }

    // Verify installation
    if !exec.check_command_exists("openssl")? {
        anyhow::bail!("openssl installation failed. Please install it manually.");
    }

    println!("✓ openssl installed successfully");
    Ok(())
}

/// Prompt for multiple choice
fn prompt_choice(question: &str, options: &[&str], default: usize) -> Result<usize> {
    println!("{}", question);
    for (i, option) in options.iter().enumerate() {
        let marker = if i == default { ">" } else { " " };
        println!("  {} {}. {}", marker, i + 1, option);
    }
    print!("Enter number [{}]: ", default + 1);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().lock().read_line(&mut input)?;
    let input = input.trim();

    if input.is_empty() {
        return Ok(default);
    }

    input
        .parse::<usize>()
        .ok()
        .and_then(|n| {
            if n >= 1 && n <= options.len() {
                Some(n - 1)
            } else {
                None
            }
        })
        .ok_or_else(|| anyhow::anyhow!("Invalid choice"))
}
