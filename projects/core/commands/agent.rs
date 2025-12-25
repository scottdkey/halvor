use crate::agent::{discovery::HostDiscovery, server::AgentServer, sync::ConfigSync};
use crate::config::service::get_current_hostname;
use anyhow::Result;
use clap::Subcommand;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Subcommand, Clone)]
pub enum AgentCommands {
    /// Start the halvor agent daemon
    Start {
        /// Port to listen on (default: 13500)
        #[arg(long, default_value = "13500")]
        port: u16,
        /// Also start web server on this port (serves UI and API)
        #[arg(long)]
        web_port: Option<u16>,
        /// Run as daemon in background
        #[arg(long)]
        daemon: bool,
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
    /// View agent logs
    Logs {
        /// Follow log output (like tail -f)
        #[arg(long, short = 'f')]
        follow: bool,
    },
}

/// Handle agent commands
pub async fn handle_agent(command: AgentCommands) -> Result<()> {
    match command {
        AgentCommands::Start {
            port,
            web_port,
            daemon,
        } => {
            start_agent(port, web_port, daemon).await?;
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
        AgentCommands::Logs { follow } => {
            show_agent_logs(follow)?;
        }
    }
    Ok(())
}

/// Start the agent daemon
async fn start_agent(port: u16, web_port: Option<u16>, daemon: bool) -> Result<()> {
    use std::fs;

    // Check if already running
    if is_agent_running()? {
        println!("Agent is already running");
        return Ok(());
    }

    if daemon {
        // Daemon mode - spawn as background process
        #[cfg(unix)]
        {
            use std::process::Command;

            let log_file = get_agent_log_file()?;
            if let Some(parent) = log_file.parent() {
                fs::create_dir_all(parent)?;
            }

            // Spawn agent in background, redirecting output to log file
            let mut cmd = Command::new(std::env::current_exe()?);
            cmd.arg("agent")
                .arg("start")
                .arg("--port")
                .arg(port.to_string());
            if let Some(wp) = web_port {
                cmd.arg("--web-port").arg(wp.to_string());
            }
            let child = cmd
                .stdout(
                    fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&log_file)?,
                )
                .stderr(
                    fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&log_file)?,
                )
                .spawn()
                .context("Failed to spawn agent daemon")?;

            // Save PID
            let pid_file = get_agent_pid_file()?;
            if let Some(parent) = pid_file.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&pid_file, child.id().to_string())?;

            println!("Agent started in daemon mode (PID: {})", child.id());
            println!("Logs: {}", log_file.display());
            println!("Use 'halvor agent logs' to view logs");
            return Ok(());
        }

        #[cfg(windows)]
        {
            anyhow::bail!(
                "Daemon mode not yet supported on Windows. Use a service manager or run without --daemon."
            );
        }
    }

    // Foreground mode - start server with background sync
    println!("Starting halvor agent on port {}...", port);
    if let Some(wp) = web_port {
        println!("Starting halvor web server on port {}...", wp);
    }

    let local_hostname = get_current_hostname()?;
    let _sync = ConfigSync::new(local_hostname.clone());

    // Spawn background sync task
    let sync_clone = ConfigSync::new(local_hostname);
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(60)); // Sync every minute
            if let Err(e) = sync_with_agents_internal(&sync_clone, false) {
                eprintln!("Background sync error: {}", e);
            }
        }
    });

    // If web_port is provided, start both agent and web server
    if let Some(web_port) = web_port {
        use crate::services::web;
        use std::net::SocketAddr;
        use std::path::PathBuf;
        use tokio::task;

        let agent_port = port;
        let server = AgentServer::new(agent_port, None);

        // Start agent server in background task
        let agent_handle = task::spawn_blocking(move || server.start());

        // Start web server in foreground
        let web_dir = std::env::var("HALVOR_WEB_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("projects/web"));
        let build_dir = web_dir.join("build");
        let static_dir = if build_dir.exists() {
            build_dir
        } else {
            web_dir.clone()
        };

        let addr = SocketAddr::from(([0, 0, 0, 0], web_port));
        println!("🌐 Web UI available at http://localhost:{}", web_port);
        println!(
            "🔌 Agent API available on port {} (for CLI connections)",
            agent_port
        );

        // Start web server (this will block)
        web::start_server(addr, static_dir, Some(agent_port)).await?;

        // If web server exits, wait for agent
        let _ = agent_handle.await;
        Ok(())
    } else {
        // Just start agent server
        let server = AgentServer::new(port, None);
        server.start()
    }
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
                    println!(
                        "  {} - {} (reachable: {})",
                        host.hostname,
                        host.tailscale_ip
                            .as_ref()
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
        println!("  - Firewall allows connections on port 13500");
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
                let ip = host
                    .tailscale_ip
                    .as_ref()
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
    let client = AgentClient::new("127.0.0.1", 13500);
    Ok(client.ping().is_ok())
}

/// Get agent PID file path
fn get_agent_pid_file() -> Result<PathBuf> {
    use crate::config::config_manager;
    let config_dir = config_manager::get_config_dir()?;
    Ok(config_dir.join("halvor-agent.pid"))
}

/// Get agent log file path
fn get_agent_log_file() -> Result<PathBuf> {
    use crate::config::config_manager;
    let config_dir = config_manager::get_config_dir()?;
    Ok(config_dir.join("halvor-agent.log"))
}

/// Show agent logs
fn show_agent_logs(follow: bool) -> Result<()> {
    let log_file = get_agent_log_file()?;

    if !log_file.exists() {
        println!("No log file found at {}", log_file.display());
        println!("Agent may not have been started in daemon mode yet.");
        return Ok(());
    }

    if follow {
        // Tail the log file continuously
        use std::fs::File;
        use std::io::{BufRead, BufReader, Seek, SeekFrom};

        let file = File::open(&log_file)?;
        let mut reader = BufReader::new(file);

        // Seek to end if file exists
        reader.seek(SeekFrom::End(0))?;

        println!("Following agent logs (Ctrl+C to stop)...");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    // No new data, wait a bit
                    std::thread::sleep(Duration::from_millis(100));
                    continue;
                }
                Ok(_) => {
                    print!("{}", line);
                    std::io::stdout().flush()?;
                }
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    // File was truncated or rotated, reopen
                    std::thread::sleep(Duration::from_millis(100));
                    let file = File::open(&log_file)?;
                    reader = BufReader::new(file);
                    reader.seek(SeekFrom::End(0))?;
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
    } else {
        // Just show the log file contents
        let contents = std::fs::read_to_string(&log_file)?;
        print!("{}", contents);
    }

    Ok(())
}
