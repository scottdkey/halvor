//! K3s cluster management commands

use crate::config;
use crate::services::k3s;
use crate::utils::exec::CommandExecutor;
use anyhow::{Context, Result};
use clap::Subcommand;

#[derive(Subcommand, Clone)]
pub enum K3sCommands {
    /// Initialize first control plane node (starts HA cluster with embedded etcd)
    Init {
        /// Token for cluster join (generated if not provided)
        #[arg(long)]
        token: Option<String>,
        /// Skip confirmation prompts
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Join a node to the cluster as control plane or agent
    Join {
        /// Target hostname to join to the cluster (can also be specified via -H/--hostname)
        #[arg(value_name = "HOSTNAME")]
        hostname: Option<String>,
        /// First control plane node address (e.g., frigg or 192.168.1.10). If not provided, will try to auto-detect from config.
        #[arg(long)]
        server: Option<String>,
        /// Cluster join token (if not provided, will be loaded from K3S_TOKEN env var or fetched from server)
        #[arg(long)]
        token: Option<String>,
        /// Join as control plane node (default: false, use --control-plane to join as control plane)
        #[arg(long, action = clap::ArgAction::SetTrue)]
        control_plane: bool,
    },
    /// Show cluster status (nodes, etcd health)
    Status,
    /// Verify HA cluster health and failover capability
    Verify {
        /// Expected node hostnames (comma-separated, e.g., "oak,frigg,baulder")
        #[arg(long)]
        nodes: String,
    },
    /// Automatically set up cluster by provisioning multiple nodes
    SetupCluster {
        /// Primary control plane node (already initialized)
        #[arg(long)]
        primary: String,
        /// Additional nodes to join as control plane (comma-separated, e.g., "frigg,baulder")
        #[arg(long)]
        nodes: String,
    },
    /// Complete cluster setup: SMB mounts, provisioning, and verification
    Setup {
        /// Primary control plane node (e.g., frigg)
        #[arg(long)]
        primary: String,
        /// Additional control plane nodes (comma-separated, e.g., "baulder,oak")
        #[arg(long)]
        nodes: String,
        /// Nodes that need SMB mounts (comma-separated, defaults to all deployment nodes)
        #[arg(long)]
        smb_nodes: Option<String>,
        /// Skip SMB mount setup
        #[arg(long)]
        skip_smb: bool,
        /// Skip SSH key setup (use if SSH is already configured)
        #[arg(long)]
        skip_ssh: bool,
        /// Skip cluster verification after setup
        #[arg(long)]
        skip_verify: bool,
    },
    /// Get kubeconfig for cluster access
    Kubeconfig {
        /// Merge into existing kubeconfig (~/.kube/config)
        #[arg(long)]
        merge: bool,
        /// Output path (default: stdout)
        #[arg(long, short = 'o')]
        output: Option<String>,
    },
    /// Uninstall K3s from a node
    Uninstall {
        /// Skip confirmation prompts
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Take an etcd snapshot for backup
    Snapshot {
        /// Output path for snapshot (default: /var/lib/rancher/k3s/server/db/snapshots/)
        #[arg(long, short = 'o')]
        output: Option<String>,
    },
    /// Restore cluster from etcd snapshot
    RestoreSnapshot {
        /// Path to snapshot file
        snapshot: String,
        /// Skip confirmation prompts
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Full cluster backup (etcd + Helm releases + secrets)
    Backup {
        /// Output directory for backup
        #[arg(long, short = 'o')]
        output: Option<String>,
        /// Include PersistentVolume data (slow, requires node access)
        #[arg(long)]
        include_pvs: bool,
    },
    /// Restore cluster from backup
    Restore {
        /// Backup directory or archive path
        backup: String,
        /// Skip confirmation prompts
        #[arg(long, short = 'y')]
        yes: bool,
        /// Only restore etcd (skip Helm releases)
        #[arg(long)]
        etcd_only: bool,
    },
    /// List available backups
    ListBackups {
        /// Directory to search for backups
        #[arg(long)]
        path: Option<String>,
    },
    /// Validate backup integrity
    ValidateBackup {
        /// Backup directory or archive path
        backup: String,
    },
}

pub fn handle_k3s(hostname: Option<&str>, command: K3sCommands) -> Result<()> {
    let halvor_dir = config::find_halvor_dir()?;
    let config = config::load_env_config(&halvor_dir)?;
    let target_host = hostname.unwrap_or("localhost");

    match command {
        K3sCommands::Init { token, yes } => {
            k3s::init_control_plane(target_host, token.as_deref(), yes, &config)?;
        }
        K3sCommands::Join {
            hostname: join_hostname,
            server,
            token,
            control_plane,
        } => {
            // Use positional hostname if provided, otherwise use global hostname
            let join_target = join_hostname.as_deref().unwrap_or(target_host);
            // Auto-detect server if not provided
            let server_addr = if let Some(s) = server {
                s
            } else {
                // First, check if we're running locally on a node with k3s
                // This handles the case where we're on frigg and want to use frigg as the server
                let mut found_primary: Option<String> = None;
                
                // First, check if we're running locally on a node with k3s
                // This handles the case where we're on frigg and want to use frigg as the server
                if target_host == "localhost" {
                    let local_exec = crate::utils::exec::Executor::Local;
                    let k3s_check = local_exec.execute_shell("systemctl is-active k3s 2>/dev/null || echo inactive").ok();
                    if let Some(check) = k3s_check {
                        let status_cow = String::from_utf8_lossy(&check.stdout);
                        let status = status_cow.trim().to_string();
                        if status == "active" {
                            // We're on a node with k3s - try to find its hostname in config
                            if let Ok(current_hostname) = crate::config::service::get_current_hostname() {
                                // Use find_hostname_in_config to normalize (handles .ts.net, etc.)
                                if let Some(normalized_hostname) = crate::config::service::find_hostname_in_config(&current_hostname, &config) {
                                    found_primary = Some(normalized_hostname);
                                }
                            }
                        }
                    }
                }
                
                // If we didn't find a local primary, check remote nodes
                // Prioritize frigg first, then check other nodes
                if found_primary.is_none() {
                    let possible_primaries = vec!["frigg", "oak", "primary"];
                    
                    for primary_name in possible_primaries {
                        // Try to get host config (this will normalize the hostname)
                        if let Ok(_host_config) = crate::services::tailscale::get_host_config(&config, primary_name) {
                            // Get the normalized hostname that was found
                            let normalized_name = crate::config::service::find_hostname_in_config(primary_name, &config)
                                .unwrap_or_else(|| primary_name.to_string());
                            
                            // Check if this node has k3s running
                            // Only try to connect if it's not the same as what we already checked locally
                            if found_primary.as_ref().map(|s| s.as_str()) != Some(&normalized_name) {
                                let exec = crate::utils::exec::Executor::new(primary_name, &config).ok();
                                if let Some(ref e) = exec {
                                    // Check if executor is local - if so, skip (we already checked)
                                    if e.is_local() {
                                        continue;
                                    }
                                    
                                    let k3s_check = e.execute_shell("systemctl is-active k3s 2>/dev/null || echo inactive").ok();
                                    if let Some(check) = k3s_check {
                                        let status_cow = String::from_utf8_lossy(&check.stdout);
                                        let status = status_cow.trim().to_string();
                                        if status == "active" {
                                            found_primary = Some(normalized_name);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                
                if let Some(primary) = found_primary {
                    println!("Auto-detected primary control plane node: {}", primary);
                    // Get Tailscale hostname from config directly
                    let host_config = crate::services::tailscale::get_host_config(&config, &primary)
                        .with_context(|| format!("Failed to get config for {}", primary))?;
                    
                    // Get Tailscale hostname from config (preferred) or construct it
                    let server_addr: String = if let Some(ts_hostname) = &host_config.hostname {
                        // Use configured Tailscale hostname
                        if ts_hostname.contains('.') {
                            ts_hostname.clone()
                        } else {
                            // Construct full hostname from tailnet base
                            format!("{}.{}", ts_hostname, config._tailnet_base)
                        }
                    } else if let Some(ts_ip) = &host_config.ip {
                        // Fallback to IP if no hostname
                        ts_ip.clone()
                    } else {
                        anyhow::bail!(
                            "No Tailscale hostname or IP configured for {} in config",
                            primary
                        );
                    };
                    
                    println!("Using server address from config: {}", server_addr);
                    server_addr
                } else {
                    anyhow::bail!(
                        "Server address not provided and could not auto-detect primary node.\n\
                         Please specify --server=<primary_node> (e.g., --server=frigg)"
                    );
                }
            };
            
            // If token not provided, get it from environment or server
            let cluster_token = if let Some(t) = token {
                t
            } else {
                // Try environment variable first (from 1Password)
                if let Ok(env_token) = std::env::var("K3S_TOKEN") {
                    println!("Using cluster token from K3S_TOKEN environment variable");
                    env_token
                } else {
                    // Fallback to getting from server node
                    println!("Fetching cluster token from {}...", server_addr);
                    let (_, fetched_token) = k3s::get_cluster_join_info(&server_addr, &config)?;
                    fetched_token
                }
            };
            k3s::join_cluster(join_target, &server_addr, &cluster_token, control_plane, &config)?;
        }
        K3sCommands::Status => {
            k3s::show_status(target_host, &config)?;
        }
        K3sCommands::Verify { nodes } => {
            let node_list: Vec<&str> = nodes.split(',').map(|s| s.trim()).collect();
            // For verify, if no hostname is provided, use the first node as the primary
            let primary_host =
                hostname.unwrap_or_else(|| node_list.first().copied().unwrap_or("localhost"));
            k3s::verify_ha_cluster(primary_host, &node_list, &config)?;
        }
        K3sCommands::SetupCluster { primary, nodes } => {
            // SetupCluster is deprecated - use Setup instead
            println!("⚠️  WARNING: 'k3s setup-cluster' is deprecated.");
            println!(
                "   Use 'halvor k3s setup --primary {} --nodes {}' instead.\n",
                primary, nodes
            );
            let node_list: Vec<&str> = nodes.split(',').map(|s| s.trim()).collect();
            k3s::setup_cluster(
                &primary, &node_list, None,  // smb_nodes
                false, // skip_smb
                false, // skip_ssh
                false, // skip_verify
                &config,
            )?;
        }
        K3sCommands::Setup {
            primary,
            nodes,
            smb_nodes,
            skip_smb,
            skip_ssh,
            skip_verify,
        } => {
            let node_list: Vec<&str> = nodes.split(',').map(|s| s.trim()).collect();
            let smb_node_list: Option<Vec<&str>> = smb_nodes
                .as_ref()
                .map(|s| s.split(',').map(|s| s.trim()).collect());
            k3s::setup_cluster(
                &primary,
                &node_list,
                smb_node_list.as_deref(),
                skip_smb,
                skip_ssh,
                skip_verify,
                &config,
            )?;
        }
        K3sCommands::Kubeconfig { merge, output } => {
            k3s::get_kubeconfig(target_host, merge, output.as_deref(), &config)?;
        }
        K3sCommands::Uninstall { yes } => {
            k3s::uninstall(target_host, yes, &config)?;
        }
        K3sCommands::Snapshot { output } => {
            k3s::take_snapshot(target_host, output.as_deref(), &config)?;
        }
        K3sCommands::RestoreSnapshot { snapshot, yes } => {
            k3s::restore_snapshot(target_host, &snapshot, yes, &config)?;
        }
        K3sCommands::Backup {
            output,
            include_pvs,
        } => {
            k3s::backup(target_host, output.as_deref(), include_pvs, &config)?;
        }
        K3sCommands::Restore {
            backup,
            yes,
            etcd_only,
        } => {
            k3s::restore(target_host, &backup, yes, etcd_only, &config)?;
        }
        K3sCommands::ListBackups { path } => {
            k3s::list_backups(path.as_deref())?;
        }
        K3sCommands::ValidateBackup { backup } => {
            k3s::validate_backup(&backup)?;
        }
    }

    Ok(())
}
