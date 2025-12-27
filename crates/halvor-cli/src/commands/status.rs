//! Status commands for various services

use halvor_core::config;
use halvor_core::services::helm;
use halvor_agent::apps::{k3s, tailscale};
use halvor_core::utils::exec::CommandExecutor;
use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand, Clone)]
pub enum StatusCommands {
    /// Show K3s cluster status (nodes, etcd health)
    K3s,
    /// List Helm releases
    Helm {
        /// Show releases in all namespaces
        #[arg(long, short = 'A')]
        all_namespaces: bool,
        /// Filter by namespace
        #[arg(long, short = 'n')]
        namespace: Option<String>,
    },
    /// Show Tailscale nodes available on the tailnet
    Tailscale,
}

/// Handle status commands
pub fn handle_status(hostname: Option<&str>, command: Option<StatusCommands>) -> Result<()> {
    let halvor_dir = config::find_halvor_dir()?;
    let config = config::load_env_config(&halvor_dir)?;

    // Detect current machine's hostname if not provided
    // This ensures we can find the host in config or use local execution
    let target_host = if let Some(host) = hostname {
        host.to_string()
    } else {
        // Try to detect current hostname and find it in config
        match halvor_core::utils::hostname::get_current_hostname() {
            Ok(current_host) => {
                // Try to find it in config (with normalization)
                if let Some(found_host) =
                    halvor_core::utils::hostname::find_hostname_in_config(&current_host, &config)
                {
                    found_host
                } else {
                    // Not in config, but we can still use it - Executor will detect it's local
                    current_host
                }
            }
            Err(_) => {
                // Fallback to localhost if we can't detect hostname
                "localhost".to_string()
            }
        }
    };

    // If no subcommand provided, show comprehensive mesh status
    match command {
        None => {
            show_mesh_status(&target_host, &config)?;
        }
        Some(StatusCommands::K3s) => {
            k3s::show_status(&target_host, &config)?;
        }
        Some(StatusCommands::Helm {
            all_namespaces,
            namespace,
        }) => {
            helm::list_releases(&target_host, all_namespaces, namespace.as_deref(), &config)?;
        }
        Some(StatusCommands::Tailscale) => {
            tailscale::show_tailscale_status(&target_host, &config)?;
        }
    }

    Ok(())
}

/// Show comprehensive mesh status (Tailscale + K3s)
fn show_mesh_status(hostname: &str, config: &config::EnvConfig) -> Result<()> {
    use halvor_core::utils::exec::Executor;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Mesh Status Overview");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    let exec = Executor::new(hostname, config)?;
    let is_local = exec.is_local();

    // Show Tailscale status
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Tailscale Network");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    if !halvor_agent::apps::tailscale::is_tailscale_installed(&exec) {
        println!("✗ Tailscale is not installed on this node.");
    } else {
        // Get Tailscale status
        let status_output = if is_local {
            std::process::Command::new("tailscale")
                .args(&["status", "--json"])
                .output()
                .ok()
        } else {
            let output = exec.execute_shell("tailscale status --json 2>&1").ok();
            output.map(|o| std::process::Output {
                status: o.status,
                stdout: o.stdout,
                stderr: o.stderr,
            })
        };

        if let Some(output) = status_output {
            if output.status.success() {
                if let Ok(status_json) = serde_json::from_slice::<serde_json::Value>(&output.stdout)
                {
                    // Get current node info
                    let mut current_node_name = "unknown".to_string();
                    let mut current_node_ip: Option<String> = None;

                    if let Some(self_data) = status_json.get("Self") {
                        if let Some(dns_name) = self_data.get("DNSName").and_then(|v| v.as_str()) {
                            current_node_name = dns_name.trim_end_matches('.').to_string();
                        }
                        if let Some(ips) = self_data.get("TailscaleIPs").and_then(|v| v.as_array())
                        {
                            if let Some(ip) = ips.iter().find_map(|v| {
                                v.as_str().and_then(|s| {
                                    if s.starts_with("100.") {
                                        Some(s.to_string())
                                    } else {
                                        None
                                    }
                                })
                            }) {
                                current_node_ip = Some(ip);
                            } else if let Some(ip) = ips.first().and_then(|v| v.as_str()) {
                                current_node_ip = Some(ip.to_string());
                            }
                        }
                    }

                    println!("Current Node:");
                    println!("  Name: {}", current_node_name);
                    if let Some(ref ip) = current_node_ip {
                        println!("  IP:   {}", ip);
                    }
                    println!();

                    // Get all peers
                    let mut peers = Vec::new();
                    if let Some(peer_map) = status_json.get("Peer") {
                        if let Some(peers_obj) = peer_map.as_object() {
                            for (_, peer_data) in peers_obj {
                                let name = peer_data
                                    .get("DNSName")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown")
                                    .trim_end_matches('.')
                                    .to_string();

                                let ip = peer_data
                                    .get("TailscaleIPs")
                                    .and_then(|v| v.as_array())
                                    .and_then(|arr| {
                                        arr.iter().find_map(|v| {
                                            v.as_str().and_then(|s| {
                                                if s.starts_with("100.") {
                                                    Some(s.to_string())
                                                } else {
                                                    None
                                                }
                                            })
                                        })
                                    })
                                    .or_else(|| {
                                        peer_data
                                            .get("TailscaleIPs")
                                            .and_then(|v| v.as_array())
                                            .and_then(|arr| arr.first())
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string())
                                    });

                                peers.push((name, ip));
                            }
                        }
                    }

                    peers.sort_by(|a, b| a.0.cmp(&b.0));

                    if peers.is_empty() {
                        println!("No other nodes found on the tailnet.");
                    } else {
                        println!("Nodes on Tailnet ({}):", peers.len());
                        println!();
                        println!("  {:<40} {:<20}", "Hostname", "IP Address");
                        println!("  {}", "-".repeat(60));

                        for (name, ip) in &peers {
                            let ip_str = ip.as_deref().unwrap_or("N/A");
                            println!("  {:<40} {:<20}", name, ip_str);
                        }
                    }
                } else {
                    println!("✓ Tailscale is running (unable to parse status)");
                }
            } else {
                println!("✗ Tailscale is installed but not running or not connected.");
            }
        } else {
            println!("✗ Unable to get Tailscale status.");
        }
    }
    println!();

    // Show K3s cluster status
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("K3s Cluster");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Check if k3s is installed
    // Use tee to capture output while showing it, then read from temp file
    let version_tmp = format!("/tmp/halvor_k3s_version_{}", std::process::id());
    // Use shell_escape to properly quote the temp file path
    use halvor_core::utils::ssh::shell_escape;
    let escaped_tmp = shell_escape(&version_tmp);
    let version_cmd = format!(
        "k3s -v 2>&1 | head -1 | tee {} || echo 'not installed' | tee {}",
        escaped_tmp, escaped_tmp
    );
    let version_output = exec.execute_shell(&version_cmd).ok();

    // Read from temp file to get the version
    let version_str = exec
        .read_file(&version_tmp)
        .ok()
        .unwrap_or_else(|| {
            // Fallback: try to get from command output
            version_output
                .and_then(|v| String::from_utf8(v.stdout).ok())
                .unwrap_or_else(|| "unknown".to_string())
        })
        .trim()
        .to_string();

    // Clean up temp file (use shell_escape for safety)
    let _ = exec.execute_shell(&format!("rm -f {}", escaped_tmp));

    if version_str.contains("not installed") || version_str.is_empty() || version_str == "unknown" {
        println!("✗ K3s is not installed on this node.");
    } else {
        println!("K3s Version: {}", version_str);
        println!();

        // Check service status
        let service_status_tmp = "/tmp/k3s_service_status";
        let _ = exec.execute_shell_interactive(&format!(
            "sudo systemctl is-active k3s > {} 2>&1 || sudo systemctl is-active k3s-agent > {} 2>&1 || echo 'not_running' > {}",
            service_status_tmp, service_status_tmp, service_status_tmp
        ));

        let service_status = exec
            .read_file(service_status_tmp)
            .unwrap_or_else(|_| "unknown".to_string())
            .trim()
            .to_string();

        if service_status == "active" || service_status == "activating" {
            println!("  ✓ K3s service is running");
            println!();

            // Show nodes
            let tmp_file = "/tmp/k3s_nodes_status";
            let _ = exec.execute_shell_interactive(&format!(
                "sudo k3s kubectl get nodes -o wide > {} 2>&1 || echo 'Unable to get nodes' > {}",
                tmp_file, tmp_file
            ));
            let nodes_output = exec
                .read_file(tmp_file)
                .unwrap_or_else(|_| "Unable to get nodes".to_string());

            if nodes_output.trim() == "Unable to get nodes" || nodes_output.trim().is_empty() {
                println!("  ⚠ Unable to get nodes. The cluster may still be initializing.");
            } else {
                println!("Nodes:");
                println!("{}", nodes_output);
            }
        } else {
            println!(
                "  ✗ K3s service is not running (status: {})",
                service_status
            );
        }
    }
    println!();

    // Show Agent Mesh status
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Halvor Agent Mesh");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Check if agent is running locally
    use halvor_agent::agent::api::AgentClient;
    let agent_running = if is_local {
        let client = AgentClient::new("127.0.0.1", 13500);
        client.ping().is_ok()
    } else {
        false // For remote hosts, we'd need to check via SSH
    };

    if agent_running {
        println!("  ✓ Agent is running");
        println!();

        // Refresh Tailscale hostnames from current Tailscale status before displaying
        // This ensures we show the latest information even if database is stale
        use halvor_agent::agent::mesh;
        let _ = mesh::refresh_peer_tailscale_hostnames();

        // Get mesh peers from database
        use halvor_db::generated::agent_peers;

        match mesh::get_active_peers() {
            Ok(peers) => {
                if peers.is_empty() {
                    println!("  No peers in mesh.");
                    println!();
                    println!("  To add peers:");
                    println!("    1. Generate a token: halvor agent token");
                    println!("    2. On another machine: halvor agent join <token>");
                } else {
                    // Get detailed peer information
                    let mut peer_details = Vec::new();
                    for peer_hostname in &peers {
                        if let Ok(Some(peer_row)) = agent_peers::select_one(
                            "hostname = ?1",
                            &[&peer_hostname as &dyn rusqlite::types::ToSql],
                        ) {
                            peer_details.push((
                                peer_hostname.clone(),
                                peer_row.tailscale_ip.clone(),
                                peer_row.tailscale_hostname.clone(),
                                peer_row.last_seen_at,
                            ));
                        } else {
                            peer_details.push((peer_hostname.clone(), None, None, None));
                        }
                    }

                    peer_details.sort_by(|a, b| a.0.cmp(&b.0));

                    println!("  Mesh Peers ({}):", peer_details.len());
                    println!();
                    println!(
                        "  {:<30} {:<20} {:<30}",
                        "Hostname", "IP Address", "Tailscale Hostname"
                    );
                    println!("  {}", "-".repeat(80));

                    for (hostname, ip, ts_hostname, last_seen) in &peer_details {
                        let ip_str = ip.as_deref().unwrap_or("N/A");
                        let ts_str = ts_hostname.as_deref().unwrap_or("N/A");
                        let status = if let Some(seen) = last_seen {
                            let age = chrono::Utc::now().timestamp() - seen;
                            if age < 300 {
                                "✓"
                            } else if age < 3600 {
                                "⚠"
                            } else {
                                "✗"
                            }
                        } else {
                            "?"
                        };
                        println!(
                            "  {:<30} {:<20} {:<30} {}",
                            hostname, ip_str, ts_str, status
                        );
                    }
                }
            }
            Err(e) => {
                println!("  ⚠️  Unable to get mesh peers: {}", e);
            }
        }
    } else {
        println!("  ✗ Agent is not running");
        println!();
        println!("  To start the agent:");
        println!("    halvor agent start --daemon");
    }
    println!();

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("For detailed information:");
    println!("  halvor status tailscale  - Show detailed Tailscale status");
    println!("  halvor status k3s       - Show detailed K3s cluster status");
    println!("  halvor status helm      - Show Helm releases");
    println!("  halvor agent status     - Show detailed agent status");
    println!("  halvor agent peers      - List all mesh peers");
    println!();

    Ok(())
}
