//! Agent installation and diagnostics service

use crate::agent::api::AgentClient;
use crate::config::EnvConfig;
use crate::services::k3s;
use crate::utils::exec::{CommandExecutor, Executor};
use anyhow::{Context, Result};

/// Install or update halvor agent on a host
pub fn install_agent(hostname: &str, config: &EnvConfig) -> Result<()> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Install/Update Halvor Agent");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    let exec = Executor::new(hostname, config)
        .with_context(|| format!("Failed to create executor for hostname: {}", hostname))?;
    let is_local = exec.is_local();

    if is_local {
        println!("Target: localhost ({})", hostname);
    } else {
        println!("Target: {} (remote)", hostname);
    }
    println!();

    // Run diagnostics first
    println!("[1/4] Running diagnostics...");
    run_agent_diagnostics(&exec, hostname)?;
    println!();

    // Ensure halvor is installed
    println!("[2/4] Ensuring halvor is installed...");
    if !is_local {
        k3s::check_and_install_halvor(&exec)?;
    } else {
        println!("  ✓ Running on localhost - halvor is available");
    }
    println!();

    // Install/update agent service
    println!("[3/4] Installing/updating agent service...");
    k3s::setup_agent_service(&exec, None)?;
    println!();

    // Verify installation
    println!("[4/4] Verifying installation...");
    verify_agent_installation(&exec, hostname)?;
    println!();

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✓ Halvor agent installation complete!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    Ok(())
}

/// Run comprehensive diagnostics for halvor agent
fn run_agent_diagnostics<E: CommandExecutor>(exec: &E, hostname: &str) -> Result<()> {
    println!("  Checking agent status...");

    // Check if systemd service exists
    let service_exists = exec
        .file_exists("/etc/systemd/system/halvor-agent.service")
        .unwrap_or(false);

    if service_exists {
        println!("  ✓ Systemd service file exists");
    } else {
        println!("  ⚠️  Systemd service file not found");
    }

    // Check service status
    let service_status = exec
        .execute_shell("systemctl is-active halvor-agent.service 2>/dev/null || echo inactive")
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    match service_status.as_str() {
        "active" => {
            println!("  ✓ Agent service is running");
        }
        "inactive" => {
            println!("  ⚠️  Agent service is not running");
        }
        "activating" => {
            println!("  ⚠️  Agent service is starting...");
        }
        "failed" => {
            println!("  ✗ Agent service has failed");
        }
        _ => {
            println!("  ⚠️  Agent service status: {}", service_status);
        }
    }

    // Check if service is enabled
    let service_enabled = exec
        .execute_shell("systemctl is-enabled halvor-agent.service 2>/dev/null || echo disabled")
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim() == "enabled")
        .unwrap_or(false);

    if service_enabled {
        println!("  ✓ Agent service is enabled (will start on boot)");
    } else {
        println!("  ⚠️  Agent service is not enabled (won't start on boot)");
    }

    // Check if halvor binary exists
    let halvor_exists = exec
        .execute_shell("which halvor 2>/dev/null || echo not_found")
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| !s.trim().is_empty() && !s.trim().contains("not_found"))
        .unwrap_or(false);

    if halvor_exists {
        let halvor_path = exec
            .execute_shell("which halvor")
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        println!("  ✓ Halvor binary found: {}", halvor_path);
    } else {
        println!("  ⚠️  Halvor binary not found (will be installed)");
    }

    // Check if agent is reachable via network
    if !exec.is_local() {
        // Try to get Tailscale hostname/IP for better connectivity
        let agent_host = if let Ok(Some(ts_hostname)) =
            crate::services::tailscale::get_tailscale_hostname_remote(exec)
        {
            ts_hostname
        } else if let Ok(Some(ts_ip)) = crate::services::tailscale::get_tailscale_ip_remote(exec) {
            ts_ip
        } else {
            hostname.to_string()
        };

        let client = AgentClient::new(&agent_host, 13500);
        match client.ping() {
            Ok(true) => {
                println!("  ✓ Agent is reachable on port 13500");
            }
            _ => {
                println!("  ⚠️  Agent is not reachable on port 13500");
            }
        }
    }

    Ok(())
}

/// Verify agent installation after setup
fn verify_agent_installation<E: CommandExecutor>(exec: &E, hostname: &str) -> Result<()> {
    // Wait a moment for service to start
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Check service status
    let service_status = exec
        .execute_shell("systemctl is-active halvor-agent.service 2>/dev/null || echo inactive")
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    if service_status == "active" {
        println!("  ✓ Agent service is running");

        // Try to ping agent if remote
        if !exec.is_local() {
            // Try to get Tailscale hostname/IP for better connectivity
            let agent_host = if let Ok(Some(ts_hostname)) =
                crate::services::tailscale::get_tailscale_hostname_remote(exec)
            {
                ts_hostname
            } else if let Ok(Some(ts_ip)) =
                crate::services::tailscale::get_tailscale_ip_remote(exec)
            {
                ts_ip
            } else {
                hostname.to_string()
            };

            let client = AgentClient::new(&agent_host, 13500);
            match client.ping() {
                Ok(true) => {
                    println!("  ✓ Agent is reachable and responding");
                }
                _ => {
                    println!("  ⚠️  Agent service is running but not yet reachable");
                    println!("     This may take a few more seconds...");
                }
            }
        }
    } else {
        println!("  ⚠️  Agent service status: {}", service_status);
        println!("     Check logs: systemctl status halvor-agent");
        println!("     View logs: journalctl -u halvor-agent -n 50");
    }

    Ok(())
}
