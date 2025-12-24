//! K3s cluster management commands

use crate::config;
use crate::services::k3s;
use anyhow::Result;
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
        /// First control plane node address (e.g., oak or 192.168.1.10)
        #[arg(long)]
        server: String,
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
            server,
            token,
            control_plane,
        } => {
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
                    println!("Fetching cluster token from {}...", server);
                    let (_, fetched_token) = k3s::get_cluster_join_info(&server, &config)?;
                    fetched_token
                }
            };
            k3s::join_cluster(target_host, &server, &cluster_token, control_plane, &config)?;
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
