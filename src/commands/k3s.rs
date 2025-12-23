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
        /// Cluster join token
        #[arg(long)]
        token: String,
        /// Join as control plane node (default: true for HA)
        #[arg(long, default_value = "true")]
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
            k3s::join_cluster(target_host, &server, &token, control_plane, &config)?;
        }
        K3sCommands::Status => {
            k3s::show_status(target_host, &config)?;
        }
        K3sCommands::Verify { nodes } => {
            let node_list: Vec<&str> = nodes.split(',').map(|s| s.trim()).collect();
            k3s::verify_ha_cluster(target_host, &node_list, &config)?;
        }
        K3sCommands::SetupCluster { primary, nodes } => {
            use crate::services::provision;
            let node_list: Vec<&str> = nodes.split(',').map(|s| s.trim()).collect();
            provision::provision_cluster_nodes(&primary, &node_list, &config)?;
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
    }

    Ok(())
}
