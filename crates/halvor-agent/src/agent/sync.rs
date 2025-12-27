use crate::agent::api::AgentClient;
use crate::agent::discovery::DiscoveredHost;
use halvor_core::services::host;
use anyhow::Result;
use uuid::Uuid;

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
        use crate::agent::mesh;

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

            // Sync host configs and mesh peers from remote
            if let Ok(sync_data_str) = client.sync_database(&self.local_hostname, None) {
                if let Ok(sync_data) = serde_json::from_str::<serde_json::Value>(&sync_data_str) {
                    // Sync host configs (write to .env)
                    if let Some(hosts_json) = sync_data.get("hosts") {
                        if let Some(hosts_map) = hosts_json.as_object() {
                            for (hostname, config_json) in hosts_map {
                                if let Ok(config) =
                                    serde_json::from_value::<halvor_core::config::HostConfig>(
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

                    // Sync mesh peers - self-healing: add any peers we don't know about
                    if let Some(peers_json) = sync_data.get("mesh_peers") {
                        if let Some(peers_array) = peers_json.as_array() {
                            for peer_json in peers_array {
                                let peer_hostname = match peer_json.get("hostname")
                                    .and_then(|v| v.as_str())
                                {
                                    Some(name) => name,
                                    None => continue, // Skip peers without hostname
                                };

                                // Check if we already have this peer
                                let existing_peers = mesh::get_active_peers().unwrap_or_default();
                                let normalized_peer = halvor_core::utils::hostname::normalize_hostname(peer_hostname);
                                let normalized_local = halvor_core::utils::hostname::normalize_hostname(&self.local_hostname);
                                
                                // Skip self
                                if normalized_peer == normalized_local {
                                    continue;
                                }

                                // Check if peer already exists
                                let peer_exists = existing_peers.iter().any(|p| {
                                    halvor_core::utils::hostname::normalize_hostname(p) == normalized_peer
                                });

                                if !peer_exists {
                                    // Add missing peer to local database (self-healing)
                                    let tailscale_ip = peer_json.get("tailscale_ip")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string());
                                    let tailscale_hostname = peer_json.get("tailscale_hostname")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string());
                                    let public_key = peer_json.get("public_key")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string())
                                        .unwrap_or_else(|| format!("pk_{}", Uuid::new_v4()));
                                    
                                    // Generate a temporary shared secret (will be updated on next sync)
                                    let shared_secret = format!("sync_secret_{}", Uuid::new_v4());

                                    if let Err(e) = mesh::add_peer(
                                        &normalized_peer,
                                        tailscale_ip,
                                        tailscale_hostname,
                                        &public_key,
                                        &shared_secret,
                                    ) {
                                        eprintln!("  Warning: Failed to add peer {}: {}", normalized_peer, e);
                                    } else {
                                        eprintln!("  ✓ Added missing peer: {} (self-healed)", normalized_peer);
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
