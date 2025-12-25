//! Join a node to the K3s cluster

use crate::config;
use crate::services::k3s;
use crate::utils::exec::CommandExecutor;
use anyhow::{Context, Result};

/// Join a node to the K3s cluster
pub fn handle_join(
    hostname: Option<&str>,
    join_hostname: Option<String>,
    server: Option<String>,
    token: Option<String>,
    control_plane: bool,
) -> Result<()> {
    let halvor_dir = config::find_halvor_dir()?;
    let config = config::load_env_config(&halvor_dir)?;
    let target_host = hostname.unwrap_or("localhost");

    // Use positional hostname if provided, otherwise use global hostname
    let join_target = join_hostname.as_deref().unwrap_or(target_host);

    // Auto-detect server if not provided
    let server_addr = if let Some(s) = server {
        s
    } else {
        auto_detect_primary_node(&config, target_host)?
    };

    // If token not provided, get it from environment or server
    let cluster_token = if let Some(t) = token {
        t
    } else {
        // Try environment variable first (from 1Password)
        if let Ok(env_token) = std::env::var("K3S_TOKEN") {
            println!("Using cluster token from K3S_TOKEN environment variable");
            env_token
        } else {
            // Fallback to getting from server node
            println!("Fetching cluster token from {}...", server_addr);
            let (_, fetched_token) = k3s::get_cluster_join_info(&server_addr, &config)?;
            fetched_token
        }
    };

    k3s::join_cluster(join_target, &server_addr, &cluster_token, control_plane, &config)?;
    Ok(())
}

/// Auto-detect the primary control plane node
fn auto_detect_primary_node(config: &config::EnvConfig, target_host: &str) -> Result<String> {
    let mut found_primary: Option<String> = None;

    // First, check if we're running locally on a node with k3s
    if target_host == "localhost" {
        let local_exec = crate::utils::exec::Executor::Local;
        let k3s_check = local_exec
            .execute_shell("systemctl is-active k3s 2>/dev/null || echo inactive")
            .ok();
        if let Some(check) = k3s_check {
            let status_cow = String::from_utf8_lossy(&check.stdout);
            let status = status_cow.trim().to_string();
            if status == "active" {
                // We're on a node with k3s - try to find its hostname in config
                if let Ok(current_hostname) = crate::config::service::get_current_hostname() {
                    // Use find_hostname_in_config to normalize (handles .ts.net, etc.)
                    if let Some(normalized_hostname) =
                        crate::config::service::find_hostname_in_config(&current_hostname, config)
                    {
                        found_primary = Some(normalized_hostname);
                    }
                }
            }
        }
    }

    // If we didn't find a local primary, check all configured nodes
    if found_primary.is_none() {
        // Try all configured hosts to find one with k3s running
        for (hostname, _host_config) in &config.hosts {
            // Skip if this is the same as what we already checked locally
            if found_primary.as_ref().map(|s| s.as_str()) == Some(hostname.as_str()) {
                continue;
            }

            // Try to create an executor for this node
            let exec = crate::utils::exec::Executor::new(hostname, config).ok();
            if let Some(ref e) = exec {
                // Check if executor is local - if so, skip (we already checked)
                if e.is_local() {
                    continue;
                }

                // Check if k3s is running on this node
                let k3s_check = e
                    .execute_shell("systemctl is-active k3s 2>/dev/null || echo inactive")
                    .ok();
                if let Some(check) = k3s_check {
                    let status_cow = String::from_utf8_lossy(&check.stdout);
                    let status = status_cow.trim().to_string();
                    if status == "active" {
                        // Verify this is actually a control plane node by checking for k3s server
                        let server_check = e
                            .execute_shell(
                                "test -f /var/lib/rancher/k3s/server/node-token 2>/dev/null && echo server || echo agent",
                            )
                            .ok();
                        if let Some(server_check_output) = server_check {
                            let server_status_cow = String::from_utf8_lossy(&server_check_output.stdout);
                            let server_status = server_status_cow.trim();
                            if server_status == "server" {
                                found_primary = Some(hostname.clone());
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some(primary) = found_primary {
        println!("Auto-detected primary control plane node: {}", primary);
        // Get Tailscale hostname from config directly
        let host_config = crate::services::tailscale::get_host_config(config, &primary)
            .with_context(|| format!("Failed to get config for {}", primary))?;

        // Get Tailscale hostname from config (preferred) or construct it
        let server_addr: String = if let Some(ts_hostname) = &host_config.hostname {
            // Use configured Tailscale hostname
            if ts_hostname.contains('.') {
                ts_hostname.clone()
            } else {
                // Construct full hostname from tailnet base
                format!("{}.{}", ts_hostname, config._tailnet_base)
            }
        } else if let Some(ts_ip) = &host_config.ip {
            // Fallback to IP if no hostname
            ts_ip.clone()
        } else {
            anyhow::bail!(
                "No Tailscale hostname or IP configured for {} in config",
                primary
            );
        };

        println!("Using server address from config: {}", server_addr);
        Ok(server_addr)
    } else {
        anyhow::bail!(
            "Server address not provided and could not auto-detect primary node.\n\
             Please specify --server=<primary_node> (e.g., --server=frigg)"
        );
    }
}

