//! Join a node to the K3s cluster

use crate::config;
use halvor_core::apps::k3s;
use halvor_core::utils::exec::CommandExecutor;
use anyhow::{Context, Result};

/// Join a node to the K3s cluster
///
/// This command works in two modes:
/// 1. **Remote join**: Run from another machine (e.g., frigg) to join a remote node
///    Example: `halvor join -H baulder --server=frigg --control-plane`
/// 2. **Local join**: Run directly on the target node (e.g., baulder) to join itself
///    Example: `halvor join --server=frigg --control-plane` (when run on baulder)
///
/// The command automatically detects:
/// - If running locally (no -H flag or -H points to localhost) vs remotely
/// - The primary control plane node if --server is not provided
/// - Uses K3S_TOKEN env var if available, otherwise fetches from primary node
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
    // This allows the command to work both remotely and locally:
    // - Remote: `halvor join -H baulder` (from frigg)
    // - Local: `halvor join` (from baulder, joins localhost)
    let join_target = if let Some(host) = hostname {
        // Global -H flag takes precedence
        host
    } else if let Some(host) = join_hostname.as_deref() {
        // Use positional argument if provided
        host
    } else {
        // Default to localhost (when running on the target machine itself)
        "localhost"
    };

    // If join_target is "localhost", resolve it to the actual hostname for better logging/UX
    // This makes messages like "Joining baulder to cluster" instead of "Joining localhost"
    // Also try to find it in config to get the normalized hostname
    let resolved_hostname = if join_target == "localhost" {
        // First try to get current hostname and find it in config
        if let Ok(current_hostname) = halvor_cli::config::service::get_current_hostname() {
            // Try to find normalized hostname in config
            if let Some(normalized) = halvor_cli::config::service::find_hostname_in_config(&current_hostname, &config) {
                normalized
            } else {
                // Not in config, use current hostname as-is
                current_hostname
            }
        } else {
            "localhost".to_string()
        }
    } else {
        join_target.to_string()
    };
    
    // Validate join target is in config (unless it's localhost)
    // Use resolved_hostname for validation if it was resolved from localhost
    let target_host_for_validation = if join_target == "localhost" {
        &resolved_hostname
    } else {
        join_target
    };
    
    if target_host_for_validation != "localhost" {
        if halvor_cli::config::service::find_hostname_in_config(target_host_for_validation, &config).is_none() {
            anyhow::bail!(
                "Target host '{}' not found in config.\n\nAdd to .env:\n  HOST_{}_IP=\"<ip-address>\"\n  HOST_{}_HOSTNAME=\"<hostname>\"\n\nOr run the command locally on {} without -H flag.",
                target_host_for_validation,
                target_host_for_validation.to_uppercase(),
                target_host_for_validation.to_uppercase(),
                target_host_for_validation
            );
        }
    }
    
    let target_host = if join_target == "localhost" {
        &resolved_hostname
    } else {
        join_target
    }; // For auto-detection logic

    // Get server address from argument or KUBE_CONFIG
    let server_addr = if let Some(s) = server {
        // Server provided via argument
        // Validate server is in config or is a valid Tailscale hostname/IP
        if !s.ends_with(".ts.net") && !s.parse::<std::net::IpAddr>().is_ok() {
            // Not a Tailscale hostname or IP, check if it's in config
            if halvor_cli::config::service::find_hostname_in_config(&s, &config).is_none() {
                println!("⚠ Warning: Server '{}' not found in config. Will attempt to resolve via Tailscale.", s);
            }
        }
        s
    } else {
        // No server provided - try to extract from KUBE_CONFIG or auto-detect
        if let Ok(kubeconfig_content) = std::env::var("KUBE_CONFIG") {
            println!("Extracting cluster server from KUBE_CONFIG environment variable...");
            let (extracted_server, _) = halvor_core::apps::k3s::kubeconfig::extract_server_and_token_from_kubeconfig(&kubeconfig_content)
                .context("Failed to extract server from KUBE_CONFIG. Ensure KUBE_CONFIG environment variable is set.")?;
            println!("Using cluster server from kubeconfig: {}", extracted_server);
            extracted_server
        } else {
            // Try to auto-detect from local cluster
            auto_detect_primary_node(&config, target_host)?
        }
    };

    // Get join token - try argument, then K3S_TOKEN env var, then fetch from server
    let cluster_token = if let Some(t) = token {
        t
    } else if let Ok(env_token) = std::env::var("K3S_TOKEN") {
        println!("Using cluster join token from K3S_TOKEN environment variable");
        env_token
    } else {
        // Fetch token from server node
        println!("Fetching cluster join token from {}...", server_addr);
        let (_, fetched_token) = k3s::get_cluster_join_info(&server_addr, &config)?;
        fetched_token
    };

    // Use resolved hostname instead of "localhost" for better UX and logging
    k3s::join_cluster(&resolved_hostname, &server_addr, &cluster_token, control_plane, &config)?;
    Ok(())
}

/// Auto-detect the primary control plane node
fn auto_detect_primary_node(config: &config::EnvConfig, target_host: &str) -> Result<String> {
    let mut found_primary: Option<String> = None;

    // First, check if we're running locally on a node with k3s
    // This works regardless of target_host - we always check the current machine first
    let local_exec = halvor_core::utils::exec::Executor::Local;
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
                    if let Ok(current_hostname) = halvor_cli::config::service::get_current_hostname() {
                        // Use find_hostname_in_config to normalize (handles .ts.net, etc.)
                        if let Some(normalized_hostname) =
                            halvor_cli::config::service::find_hostname_in_config(&current_hostname, config)
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
        let local_ips = halvor_core::utils::networking::get_local_ips().unwrap_or_default();
        
        // Try all configured hosts to find one with k3s running
        for (hostname, host_config) in &config.hosts {
            // Skip if this is the same as what we already checked locally
            if found_primary.as_ref().map(|s| s.as_str()) == Some(hostname.as_str()) {
                continue;
            }
            
            // Skip the target host - we're trying to join it, not check if it's the primary
            // Normalize both hostnames for comparison (handles .ts.net suffixes, etc.)
            let normalized_target = halvor_cli::config::service::find_hostname_in_config(target_host, config)
                .unwrap_or_else(|| target_host.to_string());
            let normalized_hostname = halvor_cli::config::service::find_hostname_in_config(hostname, config)
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
            let local_exec = halvor_core::utils::exec::Executor::Local;
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
        let host_config = halvor_core::apps::tailscale::get_host_config(config, &primary)
            .with_context(|| format!("Failed to get config for {}", primary))?;

        // Get Tailscale hostname from config (preferred) or construct it
        let server_addr: String = {
            let raw_addr = if let Some(ts_hostname) = &host_config.hostname {
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
            // Strip trailing dot (DNS absolute notation) which causes SSH resolution issues
            raw_addr.trim_end_matches('.').to_string()
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

