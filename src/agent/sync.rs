use crate::agent::api::AgentClient;
use crate::agent::discovery::DiscoveredHost;
use crate::agent::server::HostInfo;
use crate::db;
use anyhow::Result;

/// Sync configuration between halvor agents
pub struct ConfigSync {
    local_hostname: String,
}

impl ConfigSync {
    pub fn new(local_hostname: String) -> Self {
        Self { local_hostname }
    }

    /// Sync host information with discovered hosts
    pub fn sync_host_info(&self, hosts: &[DiscoveredHost]) -> Result<()> {
        for host in hosts {
            if !host.reachable {
                continue;
            }

            // Get host info from remote agent
            let client = AgentClient::new(
                host.tailscale_ip
                    .as_ref()
                    .or(host.local_ip.as_ref())
                    .ok_or_else(|| anyhow::anyhow!("No IP for host {}", host.hostname))?,
                host.agent_port,
            );

            if let Ok(remote_info) = client.get_host_info() {
                // Update local database with remote host info
                db::store_host_info(
                    &remote_info.hostname,
                    remote_info.docker_version.as_deref(),
                    remote_info.tailscale_installed,
                    remote_info.portainer_installed,
                    None,
                )?;

                // Update host config with discovered addresses
                if let Some(config) = db::get_host_config(&remote_info.hostname)? {
                    let mut updated_config = config;

                    // Update IP if we discovered a new one
                    if remote_info.local_ip.is_some() && updated_config.ip.is_none() {
                        updated_config.ip = remote_info.local_ip;
                    }

                    // Update Tailscale info
                    if remote_info.tailscale_hostname.is_some()
                        && updated_config.tailscale.is_none()
                    {
                        updated_config.tailscale = remote_info.tailscale_hostname;
                    }

                    db::store_host_config(&remote_info.hostname, &updated_config)?;
                }
            }
        }

        Ok(())
    }

    /// Sync encrypted environment data
    pub fn sync_encrypted_data(&self, hosts: &[DiscoveredHost]) -> Result<()> {
        // Export local encrypted data
        let local_data = db::export_encrypted_data()?;

        for host in hosts {
            if !host.reachable {
                continue;
            }

            // TODO: Implement encrypted data sync via agent API
            // For now, this is a placeholder
        }

        Ok(())
    }
}
