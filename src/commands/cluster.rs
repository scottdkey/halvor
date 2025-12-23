//! Cluster backup and restore commands

use crate::config;
use crate::services::cluster;
use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand, Clone)]
pub enum ClusterCommands {
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

pub fn handle_cluster(hostname: Option<&str>, command: ClusterCommands) -> Result<()> {
    let halvor_dir = config::find_halvor_dir()?;
    let config = config::load_env_config(&halvor_dir)?;
    let target_host = hostname.unwrap_or("localhost");

    match command {
        ClusterCommands::Backup { output, include_pvs } => {
            cluster::backup(target_host, output.as_deref(), include_pvs, &config)?;
        }
        ClusterCommands::Restore {
            backup,
            yes,
            etcd_only,
        } => {
            cluster::restore(target_host, &backup, yes, etcd_only, &config)?;
        }
        ClusterCommands::ListBackups { path } => {
            cluster::list_backups(path.as_deref())?;
        }
        ClusterCommands::ValidateBackup { backup } => {
            cluster::validate_backup(&backup)?;
        }
    }

    Ok(())
}
