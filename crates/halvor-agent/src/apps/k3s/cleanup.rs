//! K3s cleanup and uninstall utilities

use halvor_core::utils::exec::CommandExecutor;
use anyhow::{Context, Result};
use std::io::{self, Write};

/// Clean up existing K3s installation
pub fn cleanup_existing_k3s(exec: &dyn CommandExecutor) -> Result<()> {
    cleanup_existing_k3s_with_prompt(exec, true)
}

/// Clean up existing K3s installation with optional prompt
pub fn cleanup_existing_k3s_with_prompt(
    exec: &dyn CommandExecutor,
    prompt: bool,
) -> Result<()> {
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
        .execute_shell("systemctl list-unit-files --no-pager 2>/dev/null || systemctl list-unit-files 2>/dev/null | head -100")
        .ok()
        .map(|c| {
            let output = String::from_utf8_lossy(&c.stdout);
            output.lines().any(|line| line.contains("k3s"))
        })
        .unwrap_or(false);

    if !has_k3s_server && !has_k3s_agent && !has_k3s_service {
        return Ok(()); // No existing installation
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("⚠ Found existing K3s installation on this node.");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    
    if has_k3s_server {
        println!("  - K3s server installation detected");
    }
    if has_k3s_agent {
        println!("  - K3s agent installation detected");
    }
    if has_k3s_service {
        println!("  - K3s systemd service detected");
    }
    println!();
    
    if prompt {
        print!("Do you want to remove the existing K3s installation? [y/N]: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let should_remove = input.trim().eq_ignore_ascii_case("y") || input.trim().eq_ignore_ascii_case("yes");
        
        if !should_remove {
            println!("Aborted. Existing K3s installation will not be removed.");
            anyhow::bail!("Cannot proceed with existing K3s installation. Please remove it manually or answer 'y' to remove it automatically.");
        }
        println!();
    }

    println!("Removing existing K3s installation...");

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

    // Clean up K3s completely to ensure fresh start
    // This is critical to avoid "bootstrap data already found and encrypted with different token" errors
    println!("Cleaning up K3s completely...");

    // First, try to use the official K3s uninstall script if it exists
    let uninstall_result = exec.execute_shell("test -f /usr/local/bin/k3s-uninstall.sh && echo exists || echo missing");
    let has_uninstall_script = uninstall_result
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("exists"))
        .unwrap_or(false);

    if has_uninstall_script {
        println!("  Using official K3s uninstall script...");
        let _ = exec.execute_shell_interactive("sudo /usr/local/bin/k3s-uninstall.sh 2>&1 || true");
    } else {
        println!("  No uninstall script found, manually cleaning up...");
    }

    // Ensure everything is cleaned up (even if uninstall script ran)
    let cleanup_cmd = r#"
        sudo systemctl stop k3s.service 2>/dev/null || true
        sudo systemctl stop k3s-agent.service 2>/dev/null || true
        sudo pkill -9 k3s 2>/dev/null || true
        sudo pkill -9 containerd-shim 2>/dev/null || true
        sudo pkill -9 containerd 2>/dev/null || true
        sleep 3
        # Remove K3s binaries and scripts
        sudo rm -f /usr/local/bin/k3s 2>/dev/null || true
        sudo rm -f /usr/local/bin/k3s-killall.sh 2>/dev/null || true
        sudo rm -f /usr/local/bin/k3s-uninstall.sh 2>/dev/null || true
        sudo rm -f /usr/local/bin/k3s-agent-uninstall.sh 2>/dev/null || true
        # Remove systemd service files
        sudo rm -f /etc/systemd/system/k3s.service 2>/dev/null || true
        sudo rm -f /etc/systemd/system/k3s-agent.service 2>/dev/null || true
        sudo rm -rf /etc/systemd/system/k3s.service.d 2>/dev/null || true
        sudo systemctl daemon-reload 2>/dev/null || true
        # Clean up containerd namespaces and processes
        sudo find /run/containerd -name '*k3s*' -type f -delete 2>/dev/null || true
        sudo find /run/containerd -name '*k3s*' -type d -exec rm -rf {} + 2>/dev/null || true
        # Remove K3s data directories
        sudo rm -rf /var/lib/rancher/k3s 2>/dev/null || true
        sudo rm -rf /etc/rancher/k3s 2>/dev/null || true
        sudo rm -rf /var/lib/rancher/k3s-storage 2>/dev/null || true
        sudo rm -rf /opt/k3s 2>/dev/null || true
        # Clean up any containerd data that might have K3s references
        sudo find /var/lib/containerd -name '*k3s*' -type d -exec rm -rf {} + 2>/dev/null || true
        # Clean up cgroup leftovers
        sudo systemctl reset-failed k3s.service 2>/dev/null || true
        sudo systemctl reset-failed k3s-agent.service 2>/dev/null || true
    "#;
    let _ = exec.execute_shell_interactive(cleanup_cmd);

    // Wait a moment for cleanup to complete
    std::thread::sleep(std::time::Duration::from_secs(3));
    println!("✓ Previous installation completely removed - ready for reinitialization");

    Ok(())
}
