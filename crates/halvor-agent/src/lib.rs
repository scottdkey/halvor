pub mod agent;
pub mod apps;  // Apps module (tailscale, k3s, npm, etc.)
#[cfg(not(target_arch = "wasm32"))]
pub mod ffi;

// Re-export agent modules
pub use agent::client::HalvorClient;
pub use agent::discovery::HostDiscovery;
pub use agent::server::AgentServer;
pub use agent::install::install_agent;
pub use agent::data_sync::sync_data;

use anyhow::Result;

/// Start the agent server
///
/// # Arguments
/// * `port` - Agent API port (default: 13500)
/// * `web_port` - Optional web UI port (enables UI if Some)
pub async fn start(port: u16, web_port: Option<u16>) -> Result<()> {
    let server = AgentServer::new(port, None);
    server.start()
}
