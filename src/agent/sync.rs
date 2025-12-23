use crate::agent::api::AgentClient;
use crate::agent::discovery::DiscoveredHost;
use crate::services::host;
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
                // Update host config with discovered addresses (write to .env)
                if let Some(mut config) = host::get_host_config(&remote_info.hostname)? {
                    // Update IP if we discovered a new one
                    if remote_info.local_ip.is_some() && config.ip.is_none() {
                        config.ip = remote_info.local_ip;
                    }

                    // Update hostname info (from Tailscale discovery)
                    if remote_info.tailscale_hostname.is_some() && config.hostname.is_none() {
                        config.hostname = remote_info.tailscale_hostname;
                    }

                    host::store_host_config(&remote_info.hostname, &config)?;
                }
            }
        }

        Ok(())
    }

    /// Sync encrypted environment data (from .env file)
    pub fn sync_encrypted_data(&self, hosts: &[DiscoveredHost]) -> Result<()> {
        for host in hosts {
            if !host.reachable {
                continue;
            }

            // Skip self
            if host.hostname == self.local_hostname {
                continue;
            }

            let client = AgentClient::new(
                host.tailscale_ip
                    .as_ref()
                    .or(host.local_ip.as_ref())
                    .ok_or_else(|| anyhow::anyhow!("No IP for host {}", host.hostname))?,
                host.agent_port,
            );

            // Sync host configs from remote
            if let Ok(sync_data_str) = client.sync_database(&self.local_hostname, None) {
                if let Ok(sync_data) = serde_json::from_str::<serde_json::Value>(&sync_data_str) {
                    // Sync host configs (write to .env)
                    if let Some(hosts_json) = sync_data.get("hosts") {
                        if let Some(hosts_map) = hosts_json.as_object() {
                            for (hostname, config_json) in hosts_map {
                                if let Ok(config) =
                                    serde_json::from_value::<crate::config::HostConfig>(
                                        config_json.clone(),
                                    )
                                {
                                    // Only update if we don't have this host
                                    let should_update = host::get_host_config(hostname)?.is_none();

                                    if should_update {
                                        host::store_host_config(hostname, &config)?;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
