//! Initialize K3s cluster (primary control plane node)

use crate::config;
use crate::services::k3s;
use anyhow::Result;

/// Handle init command - initialize K3s cluster or prepare node
pub fn handle_init(hostname: &str, token: Option<&str>, yes: bool, skip_k3s: bool) -> Result<()> {
    let halvor_dir = config::find_halvor_dir()?;
    let config = config::load_env_config(&halvor_dir)?;

    if skip_k3s {
        // Just prepare the node without K3s initialization
        k3s::prepare_node(hostname, &config)?;
    } else {
        // Full K3s cluster initialization
        k3s::init_control_plane(hostname, token, yes, &config)?;
    }

    Ok(())
}

