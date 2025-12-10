use crate::config;
use crate::services;
use anyhow::Result;

/// Handle install command
/// hostname: None = local, Some(hostname) = remote host
pub fn handle_install(
    hostname: Option<&str>,
    service: &str,
    edition: &str,
    host: bool,
) -> Result<()> {
    let config = config::load_config()?;
    let target_host = hostname.unwrap_or("localhost");

    match service.to_lowercase().as_str() {
        "docker" => {
            services::docker::install_docker(target_host, &config)?;
        }
        "tailscale" => {
            if target_host == "localhost" {
                services::tailscale::install_tailscale()?;
            } else {
                services::tailscale::install_tailscale_on_host(target_host, &config)?;
            }
        }
        "portainer" => {
            if host {
                services::portainer::install_portainer_host(target_host, edition, &config)?;
            } else {
                services::portainer::install_portainer_agent(target_host, edition, &config)?;
            }
        }
        "npm" => {
            anyhow::bail!(
                "NPM installation not yet implemented. Use 'halvor {} npm' to configure proxy hosts",
                target_host
            );
        }
        _ => {
            anyhow::bail!(
                "Unknown service: {}. Supported services: docker, tailscale, portainer, npm",
                service
            );
        }
    }

    Ok(())
}
