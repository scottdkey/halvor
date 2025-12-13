use crate::config;
use crate::services::tailscale;
use anyhow::Result;

#[allow(dead_code)]
pub fn handle_tailscale(hostname: &str) -> Result<()> {
    if hostname == "localhost" {
        tailscale::install_tailscale()?;
    } else {
        let config = config::load_config()?;
        tailscale::install_tailscale_on_host(hostname, &config)?;
    }
    Ok(())
}
