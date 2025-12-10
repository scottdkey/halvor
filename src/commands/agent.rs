use crate::agent::{discovery::HostDiscovery, server::AgentServer, sync::ConfigSync};
use crate::config::service::get_current_hostname;
use anyhow::{Context, Result};
use clap::Subcommand;
use std::time::Duration;

#[derive(Subcommand, Clone)]
pub enum AgentCommands {
    /// Start the halvor agent daemon
    Start {
        /// Port to listen on (default: 23500)
        #[arg(long, default_value = "23500")]
        port: u16,
        /// Run in foreground instead of daemonizing
        #[arg(long)]
        foreground: bool,
    },
    /// Stop the halvor agent daemon
    Stop,
    /// Show agent status
    Status,
    /// Discover other halvor agents on the network
    Discover {
        /// Show verbose output
        #[arg(long)]
        verbose: bool,
    },
    /// Sync configuration with discovered agents
    Sync {
        /// Force sync even if already synced recently
        #[arg(long)]
        force: bool,
    },
}

/// Handle agent commands
pub fn handle_agent(command: AgentCommands) -> Result<()> {
    match command {
        AgentCommands::Start { port, foreground } => {
            start_agent(port, foreground)?;
        }
        AgentCommands::Stop => {
            stop_agent()?;
        }
        AgentCommands::Status => {
            show_agent_status()?;
        }
        AgentCommands::Discover { verbose } => {
            discover_agents(verbose)?;
        }
        AgentCommands::Sync { force } => {
            sync_with_agents(force)?;
        }
    }
    Ok(())
}

/// Start the agent daemon
fn start_agent(port: u16, foreground: bool) -> Result<()> {
    use std::process;

    if !foreground {
        // Check if already running
        if is_agent_running()? {
            println!("Agent is already running");
            return Ok(());
        }

        // TODO: Implement proper daemonization
        // For now, just run in foreground with a note
        println!("Starting halvor agent on port {}...", port);
        println!("Note: Running in foreground mode. Use --foreground flag or implement systemd service.");
    }

    let server = AgentServer::new(port, None);
    
    // Start background sync task if in foreground mode
    if foreground {
        let local_hostname = get_current_hostname()?;
        let sync = ConfigSync::new(local_hostname);
        
        // Spawn background sync task
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(60)); // Sync every minute
                if let Err(e) = sync_with_agents_internal(&sync, false) {
                    eprintln!("Background sync error: {}", e);
                }
            }
        });
    }

    server.start()
}

/// Stop the agent daemon
fn stop_agent() -> Result<()> {
    // TODO: Implement proper process management
    println!("Agent stop not yet implemented. Use systemd or process manager to stop the agent.");
    Ok(())
}

/// Show agent status
fn show_agent_status() -> Result<()> {
    let hostname = get_current_hostname()?;
    let running = is_agent_running()?;
    
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Halvor Agent Status");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Hostname: {}", hostname);
    println!("Status: {}", if running { "Running" } else { "Stopped" });
    println!();

    if running {
        // Try to discover other agents
        let discovery = HostDiscovery::default();
        if let Ok(hosts) = discovery.discover_all() {
            println!("Discovered Agents:");
            if hosts.is_empty() {
                println!("  (none)");
            } else {
                for host in hosts {
                    println!("  {} - {} (reachable: {})", 
                        host.hostname,
                        host.tailscale_ip.as_ref()
                            .or(host.local_ip.as_ref())
                            .unwrap_or(&"unknown".to_string()),
                        host.reachable
                    );
                }
            }
        }
    }
    
    Ok(())
}

/// Discover agents on the network
fn discover_agents(verbose: bool) -> Result<()> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Discovering Halvor Agents");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    let discovery = HostDiscovery::default();
    let hosts = discovery.discover_all()?;

    if hosts.is_empty() {
        println!("No agents discovered.");
        println!();
        println!("Make sure:");
        println!("  - Agents are running on other hosts (halvor agent start)");
        println!("  - Tailscale is configured and devices are connected");
        println!("  - Firewall allows connections on port 23500");
    } else {
        println!("Discovered {} agent(s):", hosts.len());
        println!();
        for host in &hosts {
            println!("  Hostname: {}", host.hostname);
            if let Some(ref ip) = host.tailscale_ip {
                println!("    Tailscale IP: {}", ip);
            }
            if let Some(ref ip) = host.local_ip {
                println!("    Local IP: {}", ip);
            }
            if let Some(ref ts_host) = host.tailscale_hostname {
                println!("    Tailscale Hostname: {}", ts_host);
            }
            println!("    Reachable: {}", host.reachable);
            if verbose {
                // Try to get host info
                use crate::agent::api::AgentClient;
                let ip = host.tailscale_ip.as_ref()
                    .or(host.local_ip.as_ref())
                    .ok_or_else(|| anyhow::anyhow!("No IP for host"))?;
                let client = AgentClient::new(ip, host.agent_port);
                if let Ok(info) = client.get_host_info() {
                    println!("    Docker Version: {:?}", info.docker_version);
                    println!("    Tailscale Installed: {}", info.tailscale_installed);
                    println!("    Portainer Installed: {}", info.portainer_installed);
                }
            }
            println!();
        }
    }

    Ok(())
}

/// Sync configuration with discovered agents
fn sync_with_agents(force: bool) -> Result<()> {
    let local_hostname = get_current_hostname()?;
    let sync = ConfigSync::new(local_hostname);
    sync_with_agents_internal(&sync, force)
}

fn sync_with_agents_internal(sync: &ConfigSync, _force: bool) -> Result<()> {
    let discovery = HostDiscovery::default();
    let hosts = discovery.discover_all()?;

    if hosts.is_empty() {
        println!("No agents discovered. Run 'halvor agent discover' to find agents.");
        return Ok(());
    }

    println!("Syncing with {} agent(s)...", hosts.len());
    
    // Sync host information
    sync.sync_host_info(&hosts)?;
    
    // Sync encrypted data
    sync.sync_encrypted_data(&hosts)?;
    
    println!("✓ Sync complete");
    Ok(())
}

/// Check if agent is running
fn is_agent_running() -> Result<bool> {
    use crate::agent::api::AgentClient;
    
    // Try to ping localhost agent
    let client = AgentClient::new("127.0.0.1", 23500);
    Ok(client.ping().is_ok())
}

