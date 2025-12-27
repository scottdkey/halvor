use crate::services::tailscale;
use crate::utils::{format_address, networking, write_json};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredHost {
    pub hostname: String,
    pub local_ip: Option<String>,
    pub tailscale_ip: Option<String>,
    pub tailscale_hostname: Option<String>,
    pub agent_port: u16,
    pub reachable: bool,
}

/// Discover halvor agents on the network
pub struct HostDiscovery {
    agent_port: u16,
}

impl HostDiscovery {
    pub fn new(agent_port: u16) -> Self {
        Self { agent_port }
    }

    pub fn default() -> Self {
        Self { agent_port: 13500 }
    }

    /// Discover hosts via Tailscale
    pub fn discover_via_tailscale(&self) -> Result<Vec<DiscoveredHost>> {
        let mut hosts = Vec::new();

        // Get Tailscale devices
        if let Ok(devices) = tailscale::list_tailscale_devices() {
            for device in devices {
                // Try to connect to agent on each device
                if let Some(ip) = device.ip {
                    if self.check_agent_reachable(&ip) {
                        hosts.push(DiscoveredHost {
                            hostname: device.name.clone(),
                            local_ip: None,
                            tailscale_ip: Some(ip.clone()),
                            tailscale_hostname: Some(device.name.clone()),
                            agent_port: self.agent_port,
                            reachable: true,
                        });
                    }
                }
            }
        }

        Ok(hosts)
    }

    /// Discover hosts on local network
    pub fn discover_via_local_network(&self) -> Result<Vec<DiscoveredHost>> {
        let mut hosts = Vec::new();

        // Get local network IPs
        if let Ok(local_ips) = networking::get_local_ips() {
            for ip in local_ips {
                // Skip loopback
                if ip == "127.0.0.1" || ip == "::1" {
                    continue;
                }

                // Extract network prefix (assume /24 for IPv4)
                if let Some(prefix) = ip.rsplit('.').nth(1) {
                    // Scan network for halvor agents
                    // For now, just check common IPs
                    for i in 1..255 {
                        let test_ip = format!("{}.{}", prefix, i);
                        if self.check_agent_reachable(&test_ip) {
                            hosts.push(DiscoveredHost {
                                hostname: format!("host-{}", i),
                                local_ip: Some(test_ip),
                                tailscale_ip: None,
                                tailscale_hostname: None,
                                agent_port: self.agent_port,
                                reachable: true,
                            });
                        }
                    }
                }
            }
        }

        Ok(hosts)
    }

    /// Check if agent is reachable at given IP
    fn check_agent_reachable(&self, ip: &str) -> bool {
        let addr = format_address(ip, self.agent_port);
        if let Ok(mut addr_iter) = addr.to_socket_addrs() {
            if let Some(addr) = addr_iter.next() {
                if let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_secs(1)) {
                    // Try to send ping
                    let ping = serde_json::json!({
                        "Ping": {}
                    });
                    if write_json(&mut stream, &ping).is_ok() {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Discover all available hosts (Tailscale + local network)
    pub fn discover_all(&self) -> Result<Vec<DiscoveredHost>> {
        let mut hosts = Vec::new();

        // Discover via Tailscale
        if let Ok(mut tailscale_hosts) = self.discover_via_tailscale() {
            hosts.append(&mut tailscale_hosts);
        }

        // Discover via local network
        if let Ok(mut local_hosts) = self.discover_via_local_network() {
            hosts.append(&mut local_hosts);
        }

        // Deduplicate by IP
        hosts.sort_by(|a, b| {
            let a_ip = a.tailscale_ip.as_ref().or(a.local_ip.as_ref());
            let b_ip = b.tailscale_ip.as_ref().or(b.local_ip.as_ref());
            a_ip.cmp(&b_ip)
        });
        hosts.dedup_by(|a, b| {
            let a_ip = a.tailscale_ip.as_ref().or(a.local_ip.as_ref());
            let b_ip = b.tailscale_ip.as_ref().or(b.local_ip.as_ref());
            a_ip == b_ip
        });

        Ok(hosts)
    }
}
