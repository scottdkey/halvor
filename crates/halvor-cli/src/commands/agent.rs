use halvor_agent::{HostDiscovery, AgentServer, agent::sync::ConfigSync};
use halvor_core::utils::hostname::get_current_hostname;
use anyhow::{Context, Result};
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
        /// Enable web UI on the same port as agent API
        #[arg(long)]
        ui: bool,
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
    /// Generate a join token for other agents to join this mesh
    Token,
    /// Join an existing agent mesh
    Join {
        /// Join token from another agent (if not provided, will discover and prompt)
        #[arg(value_name = "TOKEN")]
        token: Option<String>,
        /// Manual host:port to connect to (e.g., "frigg:13500" or "100.64.0.1:13500")
        #[arg(long, short = 'H')]
        host: Option<String>,
    },
    /// List peers in the mesh
    Peers,
    /// Update hostname and sync across mesh
    Hostname {
        /// New hostname to use
        #[arg(value_name = "HOSTNAME")]
        new_hostname: String,
    },
}

/// Handle agent commands
pub async fn handle_agent(command: AgentCommands) -> Result<()> {
    match command {
        AgentCommands::Start {
            port,
            ui,
            daemon,
        } => {
            start_agent(port, ui, daemon).await?;
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
        AgentCommands::Token => {
            generate_join_token()?;
        }
        AgentCommands::Join { token, host } => {
            join_mesh(token, host)?;
        }
        AgentCommands::Peers => {
            list_peers()?;
        }
        AgentCommands::Hostname { new_hostname } => {
            update_hostname(&new_hostname)?;
        }
    }
    Ok(())
}

/// Start the agent daemon
async fn start_agent(port: u16, ui: bool, daemon: bool) -> Result<()> {
    use std::fs;
    use std::path::PathBuf;

    // Check if already running
    if is_agent_running()? {
        println!("Agent is already running");
        return Ok(());
    }

    // Check for web UI files only if --ui flag is provided
    let static_dir = if ui {
        let web_dir = std::env::var("HALVOR_WEB_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("projects/web"));
        let build_dir = web_dir.join("build");
        if build_dir.exists() {
            Some(build_dir)
        } else if web_dir.exists() {
            Some(web_dir)
        } else {
            anyhow::bail!("Web UI requested but web files not found. Expected at: {}", web_dir.display());
        }
    } else {
        None
    };
    let enable_web_ui = ui && static_dir.is_some();

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
            // Note: Don't pass --daemon to the spawned process, it should run in foreground
            // but we're running it in the background via spawn()
            let mut cmd = Command::new(std::env::current_exe()?);
            cmd.arg("agent")
                .arg("start")
                .arg("--port")
                .arg(port.to_string());
            if ui {
                cmd.arg("--ui");
            }
            // Don't pass --daemon flag to spawned process - it runs in foreground
            // but we spawn it in background, so it becomes a daemon
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
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    if enable_web_ui {
        println!("Starting halvor agent with web UI on port {}...", port);
    } else {
        println!("Starting halvor agent on port {}...", port);
    }
    println!();
    println!("All output will be shown below (including join requests and debug info).");
    println!("To run in background: halvor agent start --daemon");
    println!("To view daemon logs: halvor agent logs -f");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

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

    // If web UI is available, start web server on the same port (which includes agent API)
    if let Some(static_dir) = static_dir {
        use halvor_web;
        use std::net::SocketAddr;

        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        println!("🌐 Web UI and Agent API available at http://localhost:{}", port);
        println!("🔌 Agent API endpoints: http://localhost:{}/api/*", port);

        // Start web server (this will block) - it includes agent API endpoints
        halvor_web::start_server(addr, static_dir, Some(port)).await?;
        Ok(())
    } else {
        // Just start agent server (blocking, so run in spawn_blocking)
        let server = AgentServer::new(port, None);
        tokio::task::spawn_blocking(move || server.start()).await??;
        Ok(())
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
                use halvor_agent::agent::api::AgentClient;
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
    use halvor_agent::agent::mesh;

    let discovery = HostDiscovery::default();
    let hosts = discovery.discover_all()?;

    if hosts.is_empty() {
        println!("No agents discovered. Run 'halvor agent discover' to find agents.");
        return Ok(());
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Syncing with {} agent(s)...", hosts.len());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Sync host information
    println!("[1/3] Syncing host information...");
    sync.sync_host_info(&hosts)?;

    // Sync encrypted data
    println!("[2/3] Syncing encrypted data...");
    sync.sync_encrypted_data(&hosts)?;

    // Sync mesh peer information - ensure all hosts know about each other
    println!("[3/3] Syncing mesh peer information...");
    let local_hostname = get_current_hostname()?;
    let normalized_local = halvor_core::utils::hostname::normalize_hostname(&local_hostname);

    for host in &hosts {
        // Skip self
        let normalized_peer = halvor_core::utils::hostname::normalize_hostname(&host.hostname);
        if normalized_peer == normalized_local {
            continue;
        }

        // Add this peer to local database if not already present
        if let Err(_e) = mesh::add_peer(
            &normalized_peer,
            host.tailscale_ip.clone(),
            host.tailscale_hostname.clone(),
            &format!("pk_{}", uuid::Uuid::new_v4()),
            &format!("sync_secret_{}", uuid::Uuid::new_v4()),
        ) {
            // Peer might already exist, update last seen
            let _ = mesh::update_peer_last_seen(&normalized_peer);
        } else {
            println!("  ✓ Added new peer: {}", normalized_peer);
        }
    }

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✓ Sync complete - {} peers in mesh", hosts.len());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    Ok(())
}

/// Check if agent is running
fn is_agent_running() -> Result<bool> {
    use halvor_agent::agent::api::AgentClient;

    // Try to ping localhost agent
    let client = AgentClient::new("127.0.0.1", 13500);
    Ok(client.ping().is_ok())
}

/// Get agent PID file path
fn get_agent_pid_file() -> Result<PathBuf> {
    use halvor_core::config::config_manager;
    let config_dir = config_manager::get_config_dir()?;
    Ok(config_dir.join("halvor-agent.pid"))
}

/// Get agent log file path
fn get_agent_log_file() -> Result<PathBuf> {
    use halvor_core::config::config_manager;
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

/// Generate a join token for other agents to join this mesh
fn generate_join_token() -> Result<()> {
    use halvor_agent::agent::mesh;
    use halvor_agent::apps::tailscale;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Generate Join Token");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Try to get Tailscale hostname first (e.g., "mint" or "mint.bombay-pinecone.ts.net")
    // If not available, normalize system hostname (e.g., "mint.local" -> "mint")
    let hostname = halvor_agent::apps::tailscale::get_tailscale_hostname()
        .ok()
        .flatten()
        .map(|ts_hostname| {
            // Extract short hostname from Tailscale FQDN (e.g., "mint.bombay-pinecone.ts.net" -> "mint")
            ts_hostname
                .split('.')
                .next()
                .unwrap_or(&ts_hostname)
                .to_string()
        })
        .unwrap_or_else(|| {
            // Fallback: normalize system hostname
            let system_hostname = get_current_hostname().unwrap_or_else(|_| "unknown".to_string());
            halvor_core::utils::hostname::normalize_hostname(&system_hostname)
        });

    // Get Tailscale IP if available, otherwise use local IP
    let ip = tailscale::get_tailscale_ip()
        .ok()
        .flatten()
        .unwrap_or_else(|| {
            halvor_core::utils::networking::get_local_ips()
                .ok()
                .and_then(|ips| ips.into_iter().find(|ip| ip != "127.0.0.1"))
                .unwrap_or_else(|| "127.0.0.1".to_string())
        });

    let port = 13500u16; // Default agent port

    let (encoded_token, _token) = mesh::generate_join_token(&hostname, &ip, port)?;

    println!("Join token generated successfully!");
    println!();
    println!("Issuer: {} ({}:{})", hostname, ip, port);
    println!("Expires: {} hours", mesh::TOKEN_EXPIRY_HOURS);
    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("TOKEN (copy this to the joining machine):");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("{}", encoded_token);
    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("On the joining machine, run:");
    println!("  halvor agent join {}", encoded_token);
    println!();
    println!("Or run 'halvor agent join' to discover and select from available agents.");

    Ok(())
}

/// Join an existing agent mesh
fn join_mesh(token: Option<String>, host: Option<String>) -> Result<()> {
    use halvor_agent::agent::mesh::JoinToken;
    use std::io::{BufRead, BufReader};

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Join Agent Mesh");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // If token is provided, decode it to get connection info
    if let Some(ref token_str) = token {
        let decoded = JoinToken::decode(token_str)?;

        if decoded.is_expired() {
            anyhow::bail!(
                "Join token has expired. Please request a new token from the issuing agent."
            );
        }

        println!(
            "Token issued by: {} ({}:{})",
            decoded.issuer_hostname, decoded.issuer_ip, decoded.issuer_port
        );
        println!();

        return perform_join(&decoded.issuer_ip, decoded.issuer_port, token_str);
    }

    // If host is provided manually, connect to it
    if let Some(ref host_str) = host {
        let (host_addr, port) = parse_host_port(host_str)?;

        println!("Connecting to {}:{}...", host_addr, port);

        // First, we need a token from this host - prompt for it
        println!();
        println!("Enter the join token from {}:", host_addr);
        print!("> ");
        std::io::stdout().flush()?;

        let stdin = std::io::stdin();
        let mut reader = BufReader::new(stdin.lock());
        let mut token_input = String::new();
        reader.read_line(&mut token_input)?;
        let token_input = token_input.trim();

        if token_input.is_empty() {
            anyhow::bail!(
                "No token provided. Run 'halvor agent token' on the target host to generate one."
            );
        }

        return perform_join(&host_addr, port, token_input);
    }

    // No token or host provided - discover agents and let user select
    println!("Discovering available agents...");
    println!();

    let discovery = HostDiscovery::default();
    let hosts = discovery.discover_all()?;

    if hosts.is_empty() {
        println!("No agents discovered on the network.");
        println!();
        println!("Options:");
        println!("  1. Specify a host manually: halvor agent join --host frigg:13500");
        println!("  2. Use a token directly: halvor agent join <token>");
        println!("  3. Make sure the target agent is running: halvor agent start");
        return Ok(());
    }

    // Filter to only reachable hosts (excluding self)
    // Normalize hostname for comparison (e.g., "mint.local" -> "mint")
    let local_hostname = get_current_hostname()?;
    let normalized_local = halvor_core::utils::hostname::normalize_hostname(&local_hostname);
    let available_hosts: Vec<_> = hosts
        .iter()
        .filter(|h| {
            h.reachable && {
                let normalized_peer = halvor_core::utils::hostname::normalize_hostname(&h.hostname);
                normalized_peer != normalized_local && h.hostname != local_hostname
            }
        })
        .collect();

    if available_hosts.is_empty() {
        println!("No other reachable agents found.");
        println!();
        println!("Options:");
        println!("  1. Specify a host manually: halvor agent join --host frigg:13500");
        println!("  2. Use a token directly: halvor agent join <token>");
        return Ok(());
    }

    println!("Available agents:");
    println!();
    for (i, host) in available_hosts.iter().enumerate() {
        let ip = host
            .tailscale_ip
            .as_ref()
            .or(host.local_ip.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("unknown");
        let ts_name = host
            .tailscale_hostname
            .as_ref()
            .map(|s| format!(" ({})", s))
            .unwrap_or_default();
        println!("  [{}] {} - {}{}", i + 1, host.hostname, ip, ts_name);
    }
    println!();
    println!(
        "Select an agent to join (1-{}), or 'q' to quit:",
        available_hosts.len()
    );
    print!("> ");
    std::io::stdout().flush()?;

    let stdin = std::io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let mut selection = String::new();
    reader.read_line(&mut selection)?;
    let selection = selection.trim();

    if selection.eq_ignore_ascii_case("q") {
        println!("Cancelled.");
        return Ok(());
    }

    let index: usize = selection.parse().context("Invalid selection")?;
    if index < 1 || index > available_hosts.len() {
        anyhow::bail!("Selection out of range");
    }

    let selected_host = &available_hosts[index - 1];
    let ip = selected_host
        .tailscale_ip
        .as_ref()
        .or(selected_host.local_ip.as_ref())
        .ok_or_else(|| anyhow::anyhow!("No IP for selected host"))?;

    println!();
    println!("Selected: {} ({})", selected_host.hostname, ip);
    println!();
    println!(
        "Enter the join token from {} (run 'halvor agent token' on that host):",
        selected_host.hostname
    );
    print!("> ");
    std::io::stdout().flush()?;

    let mut token_input = String::new();
    reader.read_line(&mut token_input)?;
    let token_input = token_input.trim();

    if token_input.is_empty() {
        anyhow::bail!("No token provided.");
    }

    perform_join(ip, selected_host.agent_port, token_input)
}

/// Parse host:port string
fn parse_host_port(s: &str) -> Result<(String, u16)> {
    // Handle IPv6 addresses in brackets [::1]:port
    if s.starts_with('[') {
        if let Some(bracket_end) = s.find(']') {
            let host = s[1..bracket_end].to_string();
            let port = if s.len() > bracket_end + 2 && s.chars().nth(bracket_end + 1) == Some(':') {
                s[bracket_end + 2..].parse().unwrap_or(13500)
            } else {
                13500
            };
            return Ok((host, port));
        }
    }

    // Handle hostname:port or ip:port
    if let Some(colon_pos) = s.rfind(':') {
        // Check if this might be an IPv6 address without brackets (multiple colons)
        if s.matches(':').count() > 1 {
            // Treat as IPv6 address without port
            return Ok((s.to_string(), 13500));
        }
        let host = s[..colon_pos].to_string();
        let port = s[colon_pos + 1..].parse().unwrap_or(13500);
        Ok((host, port))
    } else {
        Ok((s.to_string(), 13500))
    }
}

/// Perform the actual join operation
fn perform_join(host: &str, port: u16, token: &str) -> Result<()> {
    use halvor_agent::agent::mesh::{self, JoinToken};
    use halvor_agent::agent::server::{AgentRequest, AgentResponse};
    use halvor_core::utils::{format_address, read_json, write_json};
    use std::net::{TcpStream, ToSocketAddrs};

    // Validate token format
    let decoded = JoinToken::decode(token)?;
    if decoded.is_expired() {
        anyhow::bail!("Join token has expired.");
    }

    println!("Connecting to {}:{}...", host, port);

    // Try to get Tailscale hostname first (e.g., "mint" or "mint.bombay-pinecone.ts.net")
    // If not available, normalize system hostname (e.g., "mint.local" -> "mint")
    let local_hostname = halvor_agent::apps::tailscale::get_tailscale_hostname()
        .ok()
        .flatten()
        .map(|ts_hostname| {
            // Extract short hostname from Tailscale FQDN (e.g., "mint.bombay-pinecone.ts.net" -> "mint")
            ts_hostname
                .split('.')
                .next()
                .unwrap_or(&ts_hostname)
                .to_string()
        })
        .unwrap_or_else(|| {
            // Fallback: normalize system hostname
            let system_hostname = get_current_hostname().unwrap_or_else(|_| "unknown".to_string());
            halvor_core::utils::hostname::normalize_hostname(&system_hostname)
        });

    // Generate a public key for this node (for future encrypted communication)
    let public_key = format!("pk_{}", uuid::Uuid::new_v4());

    // Send join request
    let addr = format_address(host, port);
    let socket_addr = addr
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow::anyhow!("Failed to resolve address: {}", addr))?;

    let mut stream = TcpStream::connect_timeout(&socket_addr, Duration::from_secs(10))
        .context("Failed to connect to agent")?;

    let request = AgentRequest::JoinRequest {
        join_token: token.to_string(),
        joiner_hostname: local_hostname.clone(),
        joiner_public_key: public_key,
    };

    write_json(&mut stream, &request)?;
    let response: AgentResponse = read_json(&mut stream, 8192)?;

    match response {
        AgentResponse::JoinAccepted {
            shared_secret,
            mesh_peers,
        } => {
            println!();
            println!("Successfully joined the mesh!");
            println!();
            println!(
                "Mesh peers: {}",
                if mesh_peers.is_empty() {
                    "(none yet)".to_string()
                } else {
                    mesh_peers.join(", ")
                }
            );

            // Store the issuer peer relationship locally
            mesh::add_peer(
                &decoded.issuer_hostname,
                Some(decoded.issuer_ip.clone()),
                None,
                "issuer",
                &shared_secret,
            )?;

            // Add all other mesh peers to local database
            println!();
            if !mesh_peers.is_empty() {
                println!("Adding {} mesh peer(s) to local database...", mesh_peers.len());
                for peer_hostname in &mesh_peers {
                    // Generate a placeholder shared secret for now
                    // TODO: In a production system, this should be exchanged securely
                    let peer_secret = format!("temp_secret_{}", uuid::Uuid::new_v4());

                    if let Err(e) = mesh::add_peer(
                        peer_hostname,
                        None, // Will be discovered via Tailscale
                        None,
                        &format!("pk_{}", uuid::Uuid::new_v4()),
                        &peer_secret,
                    ) {
                        eprintln!("  Warning: Failed to add peer {}: {}", peer_hostname, e);
                    } else {
                        println!("  ✓ Added peer: {}", peer_hostname);
                    }
                }
            }

            println!();
            println!("You can now sync with this mesh using: halvor agent sync");
        }
        AgentResponse::Error { message } => {
            anyhow::bail!("Join failed: {}", message);
        }
        _ => {
            anyhow::bail!("Unexpected response from agent");
        }
    }

    Ok(())
}

/// List peers in the mesh
fn list_peers() -> Result<()> {
    use halvor_agent::agent::mesh;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Mesh Peers");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    let peers = mesh::get_active_peers()?;

    if peers.is_empty() {
        println!("No peers in mesh.");
        println!();
        println!("To add peers:");
        println!("  1. Generate a token: halvor agent token");
        println!("  2. On another machine: halvor agent join <token>");
    } else {
        println!("Active peers ({}):", peers.len());
        println!();
        for peer in peers {
            println!("  - {}", peer);
        }
    }

    Ok(())
}

/// Update hostname and sync across mesh
fn update_hostname(new_hostname: &str) -> Result<()> {
    use halvor_agent::agent::api::AgentClient;
    use halvor_agent::agent::discovery::HostDiscovery;
    use halvor_agent::apps::tailscale;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Update Hostname");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Get current hostname
    let current_hostname = get_current_hostname()?;
    let normalized_current = halvor_core::utils::hostname::normalize_hostname(&current_hostname);

    // Normalize new hostname
    let normalized_new = halvor_core::utils::hostname::normalize_hostname(new_hostname);

    if normalized_current == normalized_new {
        println!("Hostname is already '{}'", normalized_new);
        return Ok(());
    }

    println!("Current hostname: {}", normalized_current);
    println!("New hostname: {}", normalized_new);
    println!();

    // Get Tailscale IP and hostname for the update
    let tailscale_ip = tailscale::get_tailscale_ip().ok().flatten();
    let tailscale_hostname = tailscale::get_tailscale_hostname().ok().flatten();

    // Update local database entry
    println!("[1/3] Updating local hostname in database...");

    // Get current peer entry
    use halvor_db::generated::agent_peers;
    let current_peer = agent_peers::select_one(
        "hostname = ?1",
        &[&normalized_current as &dyn rusqlite::types::ToSql],
    )?;

    if let Some(peer) = current_peer {
        // Update existing peer entry with new hostname
        // We need to delete the old entry and create a new one since hostname is unique
        let peer_data = halvor_db::generated::AgentPeersRowData {
            hostname: normalized_new.clone(),
            tailscale_ip: tailscale_ip.clone(),
            tailscale_hostname: tailscale_hostname.clone(),
            public_key: peer.public_key.clone(),
            status: peer.status.clone(),
            last_seen_at: peer.last_seen_at,
            joined_at: peer.joined_at,
        };

        // Delete old entry
        agent_peers::delete_by_hostname(&normalized_current)?;

        // Create new entry
        agent_peers::upsert_one(
            "hostname = ?1",
            &[&normalized_new as &dyn rusqlite::types::ToSql],
            peer_data,
        )?;

        // Update peer_keys table
        let conn = halvor_db::get_connection()?;
        conn.execute(
            "UPDATE peer_keys SET peer_hostname = ?1 WHERE peer_hostname = ?2",
            rusqlite::params![&normalized_new, &normalized_current],
        )?;

        println!("  ✓ Local database updated");
    } else {
        // No existing peer entry, create new one
        let now = chrono::Utc::now().timestamp();
        let peer_data = halvor_db::generated::AgentPeersRowData {
            hostname: normalized_new.clone(),
            tailscale_ip: tailscale_ip.clone(),
            tailscale_hostname: tailscale_hostname.clone(),
            public_key: format!("pk_{}", uuid::Uuid::new_v4()),
            status: "active".to_string(),
            last_seen_at: Some(now),
            joined_at: now,
        };

        agent_peers::upsert_one(
            "hostname = ?1",
            &[&normalized_new as &dyn rusqlite::types::ToSql],
            peer_data,
        )?;

        println!("  ✓ Local database updated (new entry)");
    }
    println!();

    // Discover and notify all peers
    println!("[2/3] Notifying peers in mesh...");
    let discovery = HostDiscovery::default();
    let hosts = discovery.discover_all()?;

    let mut notified = 0;
    let mut failed = 0;

    for host in &hosts {
        if !host.reachable {
            continue;
        }

        // Skip self
        let normalized_peer = halvor_core::utils::hostname::normalize_hostname(&host.hostname);
        if normalized_peer == normalized_current || normalized_peer == normalized_new {
            continue;
        }

        let agent_host = host
            .tailscale_ip
            .as_ref()
            .or(host.local_ip.as_ref())
            .ok_or_else(|| anyhow::anyhow!("No IP for host {}", host.hostname))?;

        let client = AgentClient::new(agent_host, host.agent_port);

        // Use sync_database to notify peer about hostname change
        // The peer will receive the updated hostname in the sync data
        match client.sync_database(&normalized_new, None) {
            Ok(_) => {
                println!("  ✓ Notified {}", host.hostname);
                notified += 1;
            }
            Err(e) => {
                println!("  ⚠️  Failed to notify {}: {}", host.hostname, e);
                failed += 1;
            }
        }
    }

    if notified == 0 && failed == 0 {
        println!("  (No other peers in mesh)");
    } else {
        println!("  Notified {} peer(s), {} failed", notified, failed);
    }
    println!();

    // Optionally update system hostname (requires sudo)
    println!("[3/3] System hostname update (optional)...");
    println!("  To update system hostname, run:");
    println!("    sudo hostnamectl set-hostname {}", normalized_new);
    println!(
        "    (or on macOS: sudo scutil --set ComputerName {})",
        normalized_new
    );
    println!();

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✓ Hostname updated to '{}'", normalized_new);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Note: You may need to restart the agent for changes to fully propagate:");
    println!("  halvor agent stop");
    println!("  halvor agent start --daemon");

    Ok(())
}
