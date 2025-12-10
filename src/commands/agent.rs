use crate::agent::{api::AgentClient, discovery::HostDiscovery, server::AgentServer, sync::ConfigSync};
use anyhow::Result;
use clap::Subcommand;
use std::env;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Subcommand)]
pub enum AgentCommands {
    /// Start the halvor agent daemon
    Start {
        /// Port to listen on (default: 23500)
        #[arg(long, default_value = "23500")]
        port: u16,
        /// Shared secret for authentication (optional)
        #[arg(long)]
        secret: Option<String>,
    },
    /// Stop the halvor agent daemon
    Stop,
    /// Check agent status
    Status,
    /// Discover other halvor agents on the network
    Discover {
        /// Show verbose information
        #[arg(long)]
        verbose: bool,
    },
    /// Sync configuration with discovered agents
    Sync {
        /// Sync host information only (skip encrypted data)
        #[arg(long)]
        hostinfo: bool,
    },
    /// Ping a remote agent
    Ping {
        /// Hostname or IP address of the agent
        host: String,
        /// Port of the agent (default: 23500)
        #[arg(long, default_value = "23500")]
        port: u16,
    },
}

static AGENT_RUNNING: AtomicBool = AtomicBool::new(false);

pub fn handle_agent(command: AgentCommands) -> Result<()> {
    match command {
        AgentCommands::Start { port, secret } => handle_start(port, secret),
        AgentCommands::Stop => handle_stop(),
        AgentCommands::Status => handle_status(),
        AgentCommands::Discover { verbose } => handle_discover(verbose),
        AgentCommands::Sync { hostinfo } => handle_sync(hostinfo),
        AgentCommands::Ping { host, port } => handle_ping(&host, port),
    }
}

fn handle_start(port: u16, secret: Option<String>) -> Result<()> {
    // Check if agent is already running
    if is_agent_running() {
        println!("Agent is already running");
        return Ok(());
    }

    println!("Starting halvor agent on port {}...", port);
    
    let server = AgentServer::new(port, secret);
    
    // Set running flag
    AGENT_RUNNING.store(true, Ordering::Relaxed);
    
    // Start the server (this blocks)
    server.start()?;
    
    Ok(())
}

fn handle_stop() -> Result<()> {
    if !is_agent_running() {
        println!("Agent is not running");
        return Ok(());
    }

    // Try to find and kill the agent process
    let output = process::Command::new("pgrep")
        .args(&["-f", "halvor agent start"])
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let pid_str = String::from_utf8_lossy(&output.stdout);
            let pid = pid_str.trim();
            if !pid.is_empty() {
                println!("Stopping agent (PID: {})...", pid);
                process::Command::new("kill")
                    .arg(pid)
                    .output()?;
                println!("✓ Agent stopped");
                AGENT_RUNNING.store(false, Ordering::Relaxed);
                return Ok(());
            }
        }
    }

    // Fallback: try pkill
    let _ = process::Command::new("pkill")
        .args(&["-f", "halvor agent start"])
        .output();

    AGENT_RUNNING.store(false, Ordering::Relaxed);
    println!("✓ Agent stopped (or was not running)");
    Ok(())
}

fn handle_status() -> Result<()> {
    if is_agent_running() {
        println!("✓ Agent is running");
        
        // Try to ping local agent
        let client = AgentClient::new("127.0.0.1", 23500);
        if client.ping().is_ok() {
            println!("  Port: 23500");
            println!("  Status: Responding");
            
            // Try to get host info
            if let Ok(info) = client.get_host_info() {
                println!("  Hostname: {}", info.hostname);
                if let Some(ip) = &info.local_ip {
                    println!("  Local IP: {}", ip);
                }
                if let Some(ts_ip) = &info.tailscale_ip {
                    println!("  Tailscale IP: {}", ts_ip);
                }
                if let Some(ts_hostname) = &info.tailscale_hostname {
                    println!("  Tailscale Hostname: {}", ts_hostname);
                }
            }
        } else {
            println!("  Status: Not responding");
        }
    } else {
        println!("✗ Agent is not running");
        println!("  Start with: halvor agent start");
    }
    
    Ok(())
}

fn handle_discover(verbose: bool) -> Result<()> {
    println!("Discovering halvor agents...");
    println!();

    let discovery = HostDiscovery::default();
    let hosts = discovery.discover_all()?;

    if hosts.is_empty() {
        println!("No agents discovered");
        return Ok(());
    }

    println!("Found {} agent(s):", hosts.len());
    println!();

    for host in &hosts {
        println!("  Hostname: {}", host.hostname);
        if let Some(ip) = &host.local_ip {
            println!("    Local IP: {}", ip);
        }
        if let Some(ts_ip) = &host.tailscale_ip {
            println!("    Tailscale IP: {}", ts_ip);
        }
        if let Some(ts_hostname) = &host.tailscale_hostname {
            println!("    Tailscale Hostname: {}", ts_hostname);
        }
        println!("    Port: {}", host.agent_port);
        println!("    Reachable: {}", if host.reachable { "✓" } else { "✗" });
        
        if verbose && host.reachable {
            let client = AgentClient::new(
                host.tailscale_ip
                    .as_ref()
                    .or(host.local_ip.as_ref())
                    .ok_or_else(|| anyhow::anyhow!("No IP for host"))?,
                host.agent_port,
            );
            
            if let Ok(info) = client.get_host_info() {
                println!("    Docker Version: {:?}", info.docker_version);
                println!("    Tailscale Installed: {}", info.tailscale_installed);
                println!("    Portainer Installed: {}", info.portainer_installed);
            }
        }
        println!();
    }

    Ok(())
}

fn handle_sync(hostinfo_only: bool) -> Result<()> {
    println!("Syncing with discovered agents...");
    println!();

    let discovery = HostDiscovery::default();
    let hosts = discovery.discover_all()?;

    if hosts.is_empty() {
        println!("No agents discovered");
        return Ok(());
    }

    let local_hostname = env::var("HOSTNAME")
        .or_else(|_| std::fs::read_to_string("/etc/hostname"))
        .unwrap_or_else(|_| "unknown".to_string())
        .trim()
        .to_string();

    let sync = ConfigSync::new(local_hostname);
    
    println!("Syncing host information...");
    sync.sync_host_info(&hosts)?;
    println!("✓ Host information synced");

    if !hostinfo_only {
        println!("Syncing encrypted data...");
        sync.sync_encrypted_data(&hosts)?;
        println!("✓ Encrypted data synced");
    }

    Ok(())
}

fn handle_ping(host: &str, port: u16) -> Result<()> {
    println!("Pinging agent at {}:{}...", host, port);
    
    let client = AgentClient::new(host, port);
    
    match client.ping() {
        Ok(true) => {
            println!("✓ Agent is reachable");
            
            // Get host info
            match client.get_host_info() {
                Ok(info) => {
                    println!();
                    println!("Host Information:");
                    println!("  Hostname: {}", info.hostname);
                    if let Some(ip) = &info.local_ip {
                        println!("  Local IP: {}", ip);
                    }
                    if let Some(ts_ip) = &info.tailscale_ip {
                        println!("  Tailscale IP: {}", ts_ip);
                    }
                    if let Some(ts_hostname) = &info.tailscale_hostname {
                        println!("  Tailscale Hostname: {}", ts_hostname);
                    }
                    if let Some(docker_ver) = &info.docker_version {
                        println!("  Docker Version: {}", docker_ver);
                    }
                    println!("  Tailscale Installed: {}", info.tailscale_installed);
                    println!("  Portainer Installed: {}", info.portainer_installed);
                }
                Err(e) => {
                    println!("  (Could not retrieve host info: {})", e);
                }
            }
        }
        Ok(false) => {
            println!("✗ Agent did not respond to ping");
        }
        Err(e) => {
            anyhow::bail!("Failed to ping agent: {}", e);
        }
    }

    Ok(())
}

fn is_agent_running() -> bool {
    // Check if process is running
    let output = process::Command::new("pgrep")
        .args(&["-f", "halvor agent start"])
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let pid = String::from_utf8_lossy(&output.stdout).trim().to_string();
            return !pid.is_empty();
        }
    }

    // Also check the atomic flag
    AGENT_RUNNING.load(Ordering::Relaxed)
}

