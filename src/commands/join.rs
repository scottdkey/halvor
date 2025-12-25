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
    
    // Determine target host: prioritize global -H flag, then positional argument, then localhost
    let join_target = if let Some(host) = hostname {
        // Global -H flag takes precedence
        host
    } else if let Some(host) = join_hostname.as_deref() {
        // Use positional argument if provided
        host
    } else {
        // Default to localhost
        "localhost"
    };
    
    let target_host = join_target; // For auto-detection logic

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
    // This works regardless of target_host - we always check the current machine first
    let local_exec = crate::utils::exec::Executor::Local;
    let k3s_check = local_exec
        .execute_shell("systemctl is-active k3s 2>/dev/null || echo inactive")
        .ok();
    if let Some(check) = k3s_check {
        let status_cow = String::from_utf8_lossy(&check.stdout);
        let status = status_cow.trim().to_string();
        if status == "active" {
            // We're on a node with k3s - verify it's a control plane and find its hostname in config
            let server_check = local_exec
                .execute_shell(
                    "test -f /var/lib/rancher/k3s/server/node-token 2>/dev/null && echo server || echo agent",
                )
                .ok();
            if let Some(server_check_output) = server_check {
                let server_status_cow = String::from_utf8_lossy(&server_check_output.stdout);
                let server_status = server_status_cow.trim();
                if server_status == "server" {
                    // This is a control plane node - find its hostname in config
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
    }

    // If we didn't find a local primary, check all configured nodes
    // But only check hosts that are local (by IP comparison) to avoid SSH prompts
    if found_primary.is_none() {
        // Get local IPs first to check if hosts are local
        let local_ips = crate::utils::networking::get_local_ips().unwrap_or_default();
        
        // Try all configured hosts to find one with k3s running
        for (hostname, host_config) in &config.hosts {
            // Skip if this is the same as what we already checked locally
            if found_primary.as_ref().map(|s| s.as_str()) == Some(hostname.as_str()) {
                continue;
            }
            
            // Skip the target host - we're trying to join it, not check if it's the primary
            // Normalize both hostnames for comparison (handles .ts.net suffixes, etc.)
            let normalized_target = crate::config::service::find_hostname_in_config(target_host, config)
                .unwrap_or_else(|| target_host.to_string());
            let normalized_hostname = crate::config::service::find_hostname_in_config(hostname, config)
                .unwrap_or_else(|| hostname.clone());
            
            if normalized_hostname == normalized_target {
                continue;
            }

            // Only check hosts that are local (by IP comparison) to avoid SSH prompts
            let is_local_host = if let Some(ip) = &host_config.ip {
                local_ips.contains(ip)
            } else {
                false
            };
            
            if !is_local_host {
                // Skip remote hosts during auto-detection to avoid SSH prompts
                continue;
            }

            // This host is local - check if k3s is running
            let local_exec = crate::utils::exec::Executor::Local;
            let k3s_check = local_exec
                .execute_shell("systemctl is-active k3s 2>/dev/null || echo inactive")
                .ok();
            if let Some(check) = k3s_check {
                let status_cow = String::from_utf8_lossy(&check.stdout);
                let status = status_cow.trim().to_string();
                if status == "active" {
                    // Verify this is actually a control plane node by checking for k3s server
                    let server_check = local_exec
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

