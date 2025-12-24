//! K3s cleanup and uninstall utilities

use crate::utils::exec::CommandExecutor;
use anyhow::{Context, Result};

/// Clean up existing K3s installation
pub fn cleanup_existing_k3s<E: CommandExecutor>(exec: &E) -> Result<()> {
    // Check for existing K3s installation
    let has_k3s_server = exec
        .execute_shell("test -f /usr/local/bin/k3s-uninstall.sh && echo exists || echo not_exists")
        .ok()
        .and_then(|c| Some(String::from_utf8_lossy(&c.stdout).trim() == "exists"))
        .unwrap_or(false);

    let has_k3s_agent = exec
        .execute_shell(
            "test -f /usr/local/bin/k3s-agent-uninstall.sh && echo exists || echo not_exists",
        )
        .ok()
        .and_then(|c| Some(String::from_utf8_lossy(&c.stdout).trim() == "exists"))
        .unwrap_or(false);

    let has_k3s_service = exec
        .execute_shell("systemctl list-unit-files")
        .ok()
        .map(|c| {
            let output = String::from_utf8_lossy(&c.stdout);
            output.lines().any(|line| line.contains("k3s"))
        })
        .unwrap_or(false);

    if !has_k3s_server && !has_k3s_agent && !has_k3s_service {
        return Ok(()); // No existing installation
    }

    println!("⚠ Found existing K3s installation on this node.");
    println!("   Reinitializing K3s for the cluster...");

    // Uninstall existing installation
    if has_k3s_server {
        let uninstall_check = exec
            .execute_shell("test -f /usr/local/bin/k3s-uninstall.sh && echo exists")
            .ok();
        if let Some(check) = uninstall_check {
            if String::from_utf8_lossy(&check.stdout).trim() == "exists" {
                println!("Uninstalling existing K3s server...");
                exec.execute_interactive("bash", &["/usr/local/bin/k3s-uninstall.sh"])
                    .context("Failed to uninstall existing K3s server")?;
            }
        }
    } else if has_k3s_agent {
        let uninstall_check = exec
            .execute_shell("test -f /usr/local/bin/k3s-agent-uninstall.sh && echo exists")
            .ok();
        if let Some(check) = uninstall_check {
            if String::from_utf8_lossy(&check.stdout).trim() == "exists" {
                println!("Uninstalling existing K3s agent...");
                exec.execute_interactive("bash", &["/usr/local/bin/k3s-agent-uninstall.sh"])
                    .context("Failed to uninstall existing K3s agent")?;
            }
        }
    }

    // Stop and disable the service to ensure it's fully stopped
    println!("Stopping K3s service...");
    let _ = exec.execute_shell("sudo systemctl stop k3s.service 2>/dev/null || sudo systemctl stop k3s-agent.service 2>/dev/null || true");
    let _ = exec.execute_shell("sudo systemctl disable k3s.service 2>/dev/null || sudo systemctl disable k3s-agent.service 2>/dev/null || true");

    // Check for processes still using K3s ports
    println!("Checking for processes using K3s ports...");
    let port_check = exec.execute_shell("sudo lsof -i :6443 -i :10250 -i :8472 -i :2379 -i :2380 2>/dev/null || echo 'no_conflicts'").ok();
    if let Some(check) = port_check {
        let port_output = String::from_utf8_lossy(&check.stdout);
        if !port_output.contains("no_conflicts") && !port_output.trim().is_empty() {
            println!("⚠ Warning: Found processes using K3s ports:");
            println!("{}", port_output);
            println!("Attempting to kill processes...");
            let _ = exec.execute_shell("sudo pkill -9 k3s 2>/dev/null || true");
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
    }

    // Clean up K3s data directories to ensure fresh start
    println!("Cleaning up K3s data directories...");
    let _ =
        exec.execute_shell("sudo rm -rf /var/lib/rancher/k3s /etc/rancher/k3s 2>/dev/null || true");

    // Wait a moment for cleanup
    std::thread::sleep(std::time::Duration::from_secs(3));
    println!("✓ Previous installation removed - ready for reinitialization");

    Ok(())
}
