use crate::agent::discovery::DiscoveredHost;
use crate::agent::server::HostInfo;
use crate::agent::{api::AgentClient, discovery::HostDiscovery};
use anyhow::Result;

/// Client for discovering and interacting with Halvor agents
pub struct HalvorClient {
    discovery: HostDiscovery,
}

impl HalvorClient {
    /// Create a new Halvor client
    pub fn new(agent_port: Option<u16>) -> Self {
        let discovery = if let Some(port) = agent_port {
            HostDiscovery::new(port)
        } else {
            HostDiscovery::default()
        };
        Self { discovery }
    }

    /// Discover all available agents on the network
    #[halvor_ffi_macro::multi_platform_export]
    pub fn discover_agents(&self) -> Result<Vec<DiscoveredHost>, String> {
        self.discovery.discover_all().map_err(|e| e.to_string())
    }

    /// Discover agents via Tailscale
    #[halvor_ffi_macro::multi_platform_export]
    pub fn discover_via_tailscale(&self) -> Result<Vec<DiscoveredHost>, String> {
        self.discovery
            .discover_via_tailscale()
            .map_err(|e| e.to_string())
    }

    /// Discover agents on local network
    #[halvor_ffi_macro::multi_platform_export]
    pub fn discover_via_local_network(&self) -> Result<Vec<DiscoveredHost>, String> {
        self.discovery
            .discover_via_local_network()
            .map_err(|e| e.to_string())
    }

    /// Ping an agent at the given address
    #[halvor_ffi_macro::multi_platform_export]
    pub fn ping_agent(&self, host: String, port: u16) -> Result<bool, String> {
        let client = AgentClient::new(&host, port);
        client.ping().map_err(|e| e.to_string())
    }

    /// Get host information from an agent
    #[halvor_ffi_macro::multi_platform_export]
    pub fn get_host_info(&self, host: String, port: u16) -> Result<HostInfo, String> {
        let client = AgentClient::new(&host, port);
        client.get_host_info().map_err(|e| e.to_string())
    }

    /// Execute a command on a remote agent
    #[halvor_ffi_macro::multi_platform_export]
    pub fn execute_command(
        &self,
        host: String,
        port: u16,
        command: String,
        args: Vec<String>,
    ) -> Result<String, String> {
        let client = AgentClient::new(&host, port);
        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        client
            .execute_command(&command, &args_refs)
            .map_err(|e| e.to_string())
    }

    /// Get the version of the Halvor client
    /// This is a test function to verify macro generation works correctly
    #[halvor_ffi_macro::multi_platform_export]
    pub fn get_version(&self) -> Result<String, String> {
        Ok(env!("CARGO_PKG_VERSION").to_string())
    }
}
