//! Status commands for various services

use crate::config;
use crate::services::{helm, k3s};
use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand, Clone)]
pub enum StatusCommands {
    /// Show K3s cluster status (nodes, etcd health)
    K3s,
    /// List Helm releases
    Helm {
        /// Show releases in all namespaces
        #[arg(long, short = 'A')]
        all_namespaces: bool,
        /// Filter by namespace
        #[arg(long, short = 'n')]
        namespace: Option<String>,
    },
}

/// Handle status commands
pub fn handle_status(hostname: Option<&str>, command: StatusCommands) -> Result<()> {
    let halvor_dir = config::find_halvor_dir()?;
    let config = config::load_env_config(&halvor_dir)?;
    
    // Always default to localhost for status commands unless hostname is explicitly provided
    // This ensures commands run locally when on the target machine
    let target_host = hostname.unwrap_or("localhost");

    match command {
        StatusCommands::K3s => {
            k3s::show_status(target_host, &config)?;
        }
        StatusCommands::Helm {
            all_namespaces,
            namespace,
        } => {
            helm::list_releases(target_host, all_namespaces, namespace.as_deref(), &config)?;
        }
    }

    Ok(())
}

