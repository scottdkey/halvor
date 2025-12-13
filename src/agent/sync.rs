use crate::agent::api::AgentClient;
use crate::agent::discovery::DiscoveredHost;
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

    /// Sync encrypted environment data and database
    pub fn sync_encrypted_data(&self, hosts: &[DiscoveredHost]) -> Result<()> {
        use crate::db;
        use crate::db::generated::settings;

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

            // Sync database
            if let Ok(sync_data_str) = client.sync_database(&self.local_hostname, None) {
                if let Ok(sync_data) = serde_json::from_str::<serde_json::Value>(&sync_data_str) {
                    // Sync host configs
                    if let Some(hosts_json) = sync_data.get("hosts") {
                        if let Some(hosts_map) = hosts_json.as_object() {
                            for (hostname, config_json) in hosts_map {
                                if let Ok(config) =
                                    serde_json::from_value::<crate::config::HostConfig>(
                                        config_json.clone(),
                                    )
                                {
                                    // Only update if we don't have this host or if remote is newer
                                    let should_update = match db::get_host_config(hostname) {
                                        Ok(Some(_)) => {
                                            // Could add timestamp comparison here
                                            true
                                        }
                                        Ok(None) => true,
                                        Err(_) => false,
                                    };

                                    if should_update {
                                        db::store_host_config(hostname, &config)?;
                                    }
                                }
                            }
                        }
                    }

                    // Sync settings
                    if let Some(settings_json) = sync_data.get("settings") {
                        if let Some(settings_map) = settings_json.as_object() {
                            for (key, value) in settings_map {
                                if let Some(val_str) = value.as_str() {
                                    // Only sync if we don't have it or if it's different
                                    let should_update = match settings::get_setting(key) {
                                        Ok(Some(existing)) => existing != val_str,
                                        Ok(None) => true,
                                        Err(_) => false,
                                    };

                                    if should_update {
                                        settings::set_setting(key, val_str)?;
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
