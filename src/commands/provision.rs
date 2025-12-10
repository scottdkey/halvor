use crate::config;
use crate::services::provision;
use anyhow::Result;

/// Handle provision command
/// hostname: None = local, Some(hostname) = remote host
pub fn handle_provision(
    hostname: Option<&str>,
    portainer_host: bool,
    portainer_edition: &str,
) -> Result<()> {
    let config = config::load_config()?;
    let target_host = hostname.unwrap_or("localhost");
    provision::provision_host(target_host, portainer_host, portainer_edition, &config)?;
    Ok(())
}
