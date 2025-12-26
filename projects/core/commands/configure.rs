//! Configure command handlers

use crate::config;
use crate::services::k3s;
use anyhow::Result;

/// Handle configure command
pub fn handle_configure(hostname: Option<&str>) -> Result<()> {
    let halvor_dir = config::find_halvor_dir()?;
    let config = config::load_env_config(&halvor_dir)?;

    // Default to localhost if not provided
    let target_host = hostname.unwrap_or("localhost");

    // Configure Tailscale for K3s and regenerate certificates
    // This is a combined operation because updating TLS SANs requires cert regeneration
    k3s::regenerate_certificates(target_host, false, &config)?;

    Ok(())
}

