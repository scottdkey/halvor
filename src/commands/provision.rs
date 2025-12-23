//! Provision Command
//!
//! Guided setup for new hosts in the Kubernetes cluster.
//!
//! Usage:
//!   halvor provision              # Interactive guided setup on localhost
//!   halvor provision -H <host>    # Interactive guided setup on remote host
//!   halvor provision -y           # Non-interactive with defaults (requires cluster role flags)

use crate::config;
use crate::services::provision::{self, ClusterRole};
use anyhow::Result;

/// Handle provision command
pub fn handle_provision(
    hostname: Option<&str>,
    skip_prompts: bool,
    cluster_role: Option<&str>,
    cluster_server: Option<&str>,
    cluster_token: Option<&str>,
) -> Result<()> {
    let config = config::load_config()?;
    let target_host = hostname.unwrap_or("localhost");

    if skip_prompts {
        // Non-interactive mode - requires cluster role
        let role = match cluster_role {
            Some("init") => ClusterRole::InitControlPlane,
            Some("control-plane") | Some("cp") => ClusterRole::JoinControlPlane,
            Some("agent") | Some("worker") => ClusterRole::JoinAgent,
            Some(r) => anyhow::bail!(
                "Invalid cluster role: {}. Use 'init', 'control-plane', or 'agent'",
                r
            ),
            None => anyhow::bail!(
                "--cluster-role is required in non-interactive mode. Use 'init', 'control-plane', or 'agent'"
            ),
        };
        provision::provision_defaults(target_host, role, cluster_server, cluster_token, &config)?;
    } else {
        // Interactive guided mode
        provision::provision_guided(target_host, &config)?;
    }

    Ok(())
}
