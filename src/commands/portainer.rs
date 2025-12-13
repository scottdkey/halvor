use crate::config;
use crate::services::portainer;
use anyhow::Result;

#[allow(dead_code)]
pub fn handle_portainer(hostname: &str, edition: &str, host: bool) -> Result<()> {
    let config = config::load_config()?;
    if host {
        portainer::install_portainer_host(hostname, edition, &config)?;
    } else {
        portainer::install_portainer_agent(hostname, edition, &config)?;
    }
    Ok(())
}
