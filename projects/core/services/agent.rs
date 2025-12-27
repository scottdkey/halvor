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

/// Detect if the target system is macOS
fn is_macos<E: CommandExecutor>(exec: &E) -> bool {
    exec.execute_shell("uname -s")
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_lowercase() == "darwin")
        .unwrap_or(false)
}

/// Run comprehensive diagnostics for halvor agent
fn run_agent_diagnostics<E: CommandExecutor>(exec: &E, hostname: &str) -> Result<()> {
    println!("  Checking agent status...");

    if is_macos(exec) {
        run_agent_diagnostics_macos(exec, hostname)
    } else {
        run_agent_diagnostics_linux(exec, hostname)
    }
}

/// Run diagnostics on macOS
fn run_agent_diagnostics_macos<E: CommandExecutor>(exec: &E, hostname: &str) -> Result<()> {
    let home_dir = exec.get_home_dir().unwrap_or_else(|_| "/Users".to_string());
    let plist_path = format!("{}/Library/LaunchAgents/com.halvor.agent.plist", home_dir);

    // Check if launchd plist exists
    let service_exists = exec.file_exists(&plist_path).unwrap_or(false);

    if service_exists {
        println!("  ✓ Launchd plist file exists");
    } else {
        println!("  ⚠️  Launchd plist file not found");
    }

    // Check if service is loaded
    let service_loaded = exec
        .execute_shell("launchctl list com.halvor.agent 2>/dev/null")
        .ok()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if service_loaded {
        // Check if it's actually running (PID column is not "-")
        let service_info = exec
            .execute_shell("launchctl list com.halvor.agent 2>/dev/null")
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();

        let parts: Vec<&str> = service_info.split_whitespace().collect();
        let pid = parts.first().unwrap_or(&"-");

        if *pid != "-" && pid.parse::<u32>().is_ok() {
            println!("  ✓ Agent service is running (PID: {})", pid);
        } else {
            println!("  ⚠️  Agent service is loaded but not running");
        }
    } else {
        println!("  ⚠️  Agent service is not loaded");
    }

    // Check if service will start on boot (RunAtLoad in plist)
    if service_exists {
        let run_at_load = exec
            .execute_shell(&format!("defaults read {} RunAtLoad 2>/dev/null || echo 0", plist_path))
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim() == "1")
            .unwrap_or(false);

        if run_at_load {
            println!("  ✓ Agent service will start on login");
        } else {
            println!("  ⚠️  Agent service will not start on login");
        }
    } else {
        println!("  ⚠️  Agent service is not configured (won't start on login)");
    }

    // Check if halvor binary exists
    check_halvor_binary(exec)?;

    // Check network reachability if remote
    if !exec.is_local() {
        check_agent_reachability(exec, hostname)?;
    }

    Ok(())
}

/// Run diagnostics on Linux
fn run_agent_diagnostics_linux<E: CommandExecutor>(exec: &E, hostname: &str) -> Result<()> {
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
    check_halvor_binary(exec)?;

    // Check network reachability if remote
    if !exec.is_local() {
        check_agent_reachability(exec, hostname)?;
    }

    Ok(())
}

/// Check if halvor binary exists
fn check_halvor_binary<E: CommandExecutor>(exec: &E) -> Result<()> {
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

    Ok(())
}

/// Check if agent is reachable via network
fn check_agent_reachability<E: CommandExecutor>(exec: &E, hostname: &str) -> Result<()> {
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

    Ok(())
}

/// Verify agent installation after setup
fn verify_agent_installation<E: CommandExecutor>(exec: &E, hostname: &str) -> Result<()> {
    // Wait a moment for service to start
    std::thread::sleep(std::time::Duration::from_secs(2));

    if is_macos(exec) {
        verify_agent_installation_macos(exec, hostname)
    } else {
        verify_agent_installation_linux(exec, hostname)
    }
}

/// Verify agent installation on macOS
fn verify_agent_installation_macos<E: CommandExecutor>(exec: &E, hostname: &str) -> Result<()> {
    let home_dir = exec.get_home_dir().unwrap_or_else(|_| "/Users".to_string());
    let log_dir = format!("{}/Library/Logs/halvor", home_dir);

    // Check if service is running
    let service_info = exec
        .execute_shell("launchctl list com.halvor.agent 2>/dev/null")
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default();

    let parts: Vec<&str> = service_info.split_whitespace().collect();
    let pid = parts.first().unwrap_or(&"-");

    if *pid != "-" && pid.parse::<u32>().is_ok() {
        println!("  ✓ Agent service is running (PID: {})", pid);

        // Try to ping agent if remote
        if !exec.is_local() {
            check_agent_ping(exec, hostname)?;
        }
    } else {
        println!("  ⚠️  Agent service is not running");
        println!("     Check logs: tail -f {}/halvor-agent.log", log_dir);
        println!(
            "     Error logs: tail -f {}/halvor-agent.error.log",
            log_dir
        );
    }

    Ok(())
}

/// Verify agent installation on Linux
fn verify_agent_installation_linux<E: CommandExecutor>(exec: &E, hostname: &str) -> Result<()> {
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
            check_agent_ping(exec, hostname)?;
        }
    } else {
        println!("  ⚠️  Agent service status: {}", service_status);
        println!("     Check logs: systemctl status halvor-agent");
        println!("     View logs: journalctl -u halvor-agent -n 50");
    }

    Ok(())
}

/// Check if agent is responding to ping
fn check_agent_ping<E: CommandExecutor>(exec: &E, hostname: &str) -> Result<()> {
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
            println!("  ✓ Agent is reachable and responding");
        }
        _ => {
            println!("  ⚠️  Agent service is running but not yet reachable");
            println!("     This may take a few more seconds...");
        }
    }

    Ok(())
}
