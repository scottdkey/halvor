//! K3s cluster status and information

use crate::config::EnvConfig;
use crate::services::tailscale;
use crate::utils::exec::{CommandExecutor, Executor};
use anyhow::Result;
use std::io::{self, Write};

/// Show cluster status
pub fn show_status(hostname: &str, config: &EnvConfig) -> Result<()> {
    let exec = Executor::new(hostname, config)?;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("K3s Cluster Status");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Authenticate sudo first so user can enter password
    println!("(You may be prompted for your sudo password)");
    io::stdout().flush()?;
    exec.execute_interactive("sudo", &["-v"])?;

    // Check if k3s is installed
    let version = exec.execute_shell(
        "k3s -v 2>/dev/null | head -1 || k3s version 2>/dev/null | head -1 || echo 'not installed'",
    )?;
    let version_str = String::from_utf8_lossy(&version.stdout);
    if version_str.contains("not installed") {
        println!("K3s is not installed on this node.");
        return Ok(());
    }
    println!("K3s Version: {}", version_str.trim());
    println!();

    // Check if K3s server service is running
    println!("Service Status:");
    let service_status_tmp = "/tmp/k3s_service_status";
    exec.execute_shell_interactive(&format!(
        "sudo systemctl is-active k3s > {} 2>&1 || sudo systemctl is-active k3s-agent > {} 2>&1 || echo 'not_running' > {}",
        service_status_tmp, service_status_tmp, service_status_tmp
    ))?;
    let service_status = exec
        .read_file(service_status_tmp)
        .unwrap_or_else(|_| "unknown".to_string());
    let service_status_trimmed = service_status.trim();

    if service_status_trimmed == "active" || service_status_trimmed == "activating" {
        println!("  ✓ K3s service is running");
    } else {
        println!(
            "  ✗ K3s service is not running (status: {})",
            service_status_trimmed
        );
        println!();
        println!("To start K3s, you need to initialize the cluster:");
        println!("  halvor init -H {} -y", hostname);
        return Ok(());
    }
    println!();

    // Show nodes - use temp file to capture output from interactive command
    println!("Nodes:");
    let tmp_file = "/tmp/k3s_nodes_status";
    exec.execute_shell_interactive(&format!(
        "sudo k3s kubectl get nodes -o wide > {} 2>&1 || echo 'Unable to get nodes' > {}",
        tmp_file, tmp_file
    ))?;
    let nodes_output = exec
        .read_file(tmp_file)
        .unwrap_or_else(|_| "Unable to get nodes".to_string());

    if nodes_output.trim() == "Unable to get nodes" || nodes_output.trim().is_empty() {
        println!("  ⚠ Unable to get nodes. The cluster may still be initializing.");
        println!();
        println!("If the cluster was just initialized, wait a few minutes and try again.");
        println!("If the cluster was never initialized, run:");
        println!("  halvor init -H {} -y", hostname);
    } else {
        println!("{}", nodes_output);
    }

    // Show etcd status for control plane nodes
    println!("etcd Status:");
    let etcd_tmp = "/tmp/k3s_etcd_status";
    exec.execute_shell_interactive(&format!(
        "sudo k3s etcd-snapshot list 2>/dev/null | head -5 > {} || echo 'Not a control plane node or etcd not available' > {}",
        etcd_tmp, etcd_tmp
    ))?;
    let etcd_output = exec
        .read_file(etcd_tmp)
        .unwrap_or_else(|_| "Not a control plane node or etcd not available".to_string());
    println!("{}", etcd_output);

    Ok(())
}

/// Get cluster join information from an existing control plane node
/// Returns (server_address, token) for joining new nodes
pub fn get_cluster_join_info(hostname: &str, config: &EnvConfig) -> Result<(String, String)> {
    let exec = Executor::new(hostname, config)?;

    // Authenticate sudo
    exec.execute_interactive("sudo", &["-v"])?;

    // Get Tailscale IP/hostname for server address
    let tailscale_ip = tailscale::get_tailscale_ip_with_fallback(&exec, hostname, config)?;
    let tailscale_hostname = tailscale::get_tailscale_hostname_remote(&exec)
        .ok()
        .flatten();

    let server_addr = tailscale_hostname.as_ref().unwrap_or(&tailscale_ip);

    // Try environment variable first (from 1Password/direnv)
    let token = if let Ok(env_token) = std::env::var("K3S_TOKEN") {
        if !env_token.trim().is_empty() {
            env_token
        } else {
            // Environment variable is empty, try reading from server
            let token_tmp = "/tmp/k3s_cluster_token";
            let token_result = exec.execute_shell_interactive(&format!(
                "sudo cat /var/lib/rancher/k3s/server/node-token > {} 2>&1 || echo '' > {}",
                token_tmp, token_tmp
            ));

            if token_result.is_ok() {
                if let Ok(token_content) = exec.read_file(token_tmp) {
                    let parsed_token = crate::services::k3s::utils::parse_node_token(&token_content);
                    if !parsed_token.is_empty() {
                        parsed_token
                    } else {
                        anyhow::bail!(
                            "Could not retrieve cluster token. K3S_TOKEN is empty and token file is empty. Please set K3S_TOKEN in 1Password or ensure the cluster is initialized."
                        )
                    }
                } else {
                    anyhow::bail!(
                        "Could not retrieve cluster token. K3S_TOKEN is empty and could not read token file. Please set K3S_TOKEN in 1Password or ensure the cluster is initialized."
                    )
                }
            } else {
                anyhow::bail!(
                    "Could not retrieve cluster token. K3S_TOKEN is empty and could not access token file. Please set K3S_TOKEN in 1Password or ensure the cluster is initialized."
                )
            }
        }
    } else {
        // No environment variable, try reading from server
        let token_tmp = "/tmp/k3s_cluster_token";
        let token_result = exec.execute_shell_interactive(&format!(
            "sudo cat /var/lib/rancher/k3s/server/node-token > {} 2>&1 || echo '' > {}",
            token_tmp, token_tmp
        ));

        if token_result.is_ok() {
            if let Ok(token_content) = exec.read_file(token_tmp) {
                let parsed_token = crate::services::k3s::utils::parse_node_token(&token_content);
                if !parsed_token.is_empty() {
                    parsed_token
                } else {
                    anyhow::bail!(
                        "Could not retrieve cluster token. Token file is empty and K3S_TOKEN is not set. Please set K3S_TOKEN in 1Password or ensure the cluster is initialized."
                    )
                }
            } else {
                anyhow::bail!(
                    "Could not retrieve cluster token. Could not read token file and K3S_TOKEN is not set. Please set K3S_TOKEN in 1Password or ensure the cluster is initialized."
                )
            }
        } else {
            anyhow::bail!(
                "Could not retrieve cluster token. Could not access token file and K3S_TOKEN is not set. Please set K3S_TOKEN in 1Password or ensure the cluster is initialized."
            )
        }
    };

    Ok((server_addr.to_string(), token))
}
