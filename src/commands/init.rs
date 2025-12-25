//! Initialize K3s cluster (primary control plane node)

use crate::config;
use crate::services::k3s;
use anyhow::Result;

/// Handle init command - initialize K3s cluster
pub fn handle_init(hostname: &str, token: Option<&str>, yes: bool) -> Result<()> {
    let halvor_dir = config::find_halvor_dir()?;
    let config = config::load_env_config(&halvor_dir)?;
    k3s::init_control_plane(hostname, token, yes, &config)?;
    Ok(())
}

