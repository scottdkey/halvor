use crate::utils::exec::CommandExecutor;
use anyhow::Result;
use serde_json;

/// Run comprehensive Docker daemon diagnostics
pub fn diagnose_docker<E: CommandExecutor>(exec: &E, hostname: &str) -> Result<()> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Docker Daemon Diagnostics");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Host: {}", hostname);
    println!();

    check_docker_installation(exec)?;
    check_daemon_accessibility(exec)?;
    check_service_status(exec)?;
    check_service_logs(exec)?;
    check_daemon_config(exec)?;
    check_socket_permissions(exec)?;
    check_user_permissions(exec)?;
    check_containerd(exec)?;
    print_summary(exec)?;
    check_network_controller_errors(exec)?;

    Ok(())
}

fn check_docker_installation<E: CommandExecutor>(exec: &E) -> Result<()> {
    println!("[1/8] Checking if Docker is installed...");
    if exec.check_command_exists("docker")? {
        let version_output = exec.execute_simple("docker", &["--version"]);
        if let Ok(output) = version_output {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("   ✓ Docker installed: {}", version);
        } else {
            println!("   ✗ Docker command found but version check failed");
        }
    } else {
        println!("   ✗ Docker is not installed");
        println!(
            "   → Install with: halvor install docker -H {}",
            exec.get_username().unwrap_or_default()
        );
        return Ok(());
    }
    println!();
    Ok(())
}

fn check_daemon_accessibility<E: CommandExecutor>(exec: &E) -> Result<()> {
    println!("[2/8] Checking Docker daemon accessibility...");
    let docker_info = exec.execute_simple("docker", &["info"]);
    match docker_info {
        Ok(output) if output.status.success() => {
            println!("   ✓ Docker daemon is accessible");
        }
        Ok(_) => {
            println!("   ✗ Docker daemon is not accessible (command failed)");
        }
        Err(e) => {
            println!("   ✗ Docker daemon is not accessible: {}", e);
        }
    }

    // Try with sudo
    let sudo_docker_info = exec.execute_simple("sudo", &["docker", "info"]);
    match sudo_docker_info {
        Ok(output) if output.status.success() => {
            println!("   ⚠ Docker works with sudo - permission issue detected");
            println!("   → User may need to be added to docker group");
        }
        _ => {}
    }
    println!();
    Ok(())
}

fn check_service_status<E: CommandExecutor>(exec: &E) -> Result<()> {
    println!("[3/8] Checking Docker service status...");
    if exec.check_command_exists("systemctl")? {
        let status_output = exec.execute_simple("systemctl", &["is-active", "docker"]);
        if let Ok(output) = status_output {
            let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
            match status.as_str() {
                "active" => println!("   ✓ Docker service is active"),
                "inactive" => println!("   ✗ Docker service is inactive"),
                "failed" => println!("   ✗ Docker service has failed"),
                _ => println!("   ⚠ Docker service status: {}", status),
            }
        }

        // Get detailed status
        let detailed_status =
            exec.execute_simple("systemctl", &["status", "docker", "--no-pager", "-l"]);
        if let Ok(output) = detailed_status {
            let status_text = String::from_utf8_lossy(&output.stdout);
            // Look for key indicators
            if status_text.contains("Active: active") {
                println!("   ✓ Service is running");
            } else if status_text.contains("Active: failed") {
                println!("   ✗ Service has failed");
            } else if status_text.contains("Active: inactive") {
                println!("   ✗ Service is not running");
            }

            // Check if enabled
            if status_text.contains("enabled") {
                println!("   ✓ Service is enabled (will start on boot)");
            } else {
                println!("   ⚠ Service is not enabled (won't start on boot)");
                println!("   → Enable with: sudo systemctl enable docker");
            }
        }
    } else {
        println!("   ⚠ systemctl not available (non-systemd system)");
    }
    println!();
    Ok(())
}

fn check_service_logs<E: CommandExecutor>(exec: &E) -> Result<()> {
    println!("[4/8] Checking Docker service logs (last 20 lines)...");
    if exec.check_command_exists("journalctl")? {
        let log_output = exec.execute_simple(
            "journalctl",
            &["-u", "docker.service", "-n", "20", "--no-pager"],
        );
        if let Ok(output) = log_output {
            let logs = String::from_utf8_lossy(&output.stdout);
            if logs.trim().is_empty() {
                println!("   ⚠ No recent logs found");
            } else {
                // Look for common error patterns
                let log_lower = logs.to_lowercase();
                if log_lower.contains("error") || log_lower.contains("failed") {
                    println!("   ✗ Errors found in logs:");
                    for line in logs.lines() {
                        if line.to_lowercase().contains("error")
                            || line.to_lowercase().contains("failed")
                        {
                            println!("      {}", line.trim());
                        }
                    }
                } else {
                    println!("   ✓ No obvious errors in recent logs");
                }
            }
        }
    } else {
        println!("   ⚠ journalctl not available");
    }
    println!();
    Ok(())
}

fn check_daemon_config<E: CommandExecutor>(exec: &E) -> Result<()> {
    println!("[5/8] Checking Docker daemon configuration...");
    let daemon_json = "/etc/docker/daemon.json";
    if exec.file_exists(daemon_json)? {
        println!("   ✓ daemon.json exists");
        let content = exec.read_file(daemon_json)?;

        // Try to parse as JSON
        if let Ok(_) = serde_json::from_str::<serde_json::Value>(&content) {
            println!("   ✓ daemon.json is valid JSON");
        } else {
            println!("   ✗ daemon.json contains invalid JSON!");
            println!("   → This may prevent Docker from starting");
            println!(
                "   → Validate with: sudo python3 -m json.tool {}",
                daemon_json
            );
        }

        // Check for backup
        if exec.file_exists(&format!("{}.backup", daemon_json))? {
            println!("   ✓ Backup exists: {}.backup", daemon_json);
        }
    } else {
        println!("   ℹ daemon.json does not exist (using defaults)");
    }
    println!();
    Ok(())
}

fn check_socket_permissions<E: CommandExecutor>(exec: &E) -> Result<()> {
    println!("[6/8] Checking Docker socket permissions...");
    let socket_path = "/var/run/docker.sock";
    if exec.file_exists(socket_path)? {
        println!("   ✓ Docker socket exists");
        // Try to get permissions (this might require sudo)
        let stat_output = exec.execute_simple("stat", &["-c", "%a %U:%G", socket_path]);
        if let Ok(output) = stat_output {
            let perms = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("   → Socket permissions: {}", perms);
        } else {
            // Try with sudo
            let sudo_stat = exec.execute_simple("sudo", &["stat", "-c", "%a %U:%G", socket_path]);
            if let Ok(output) = sudo_stat {
                let perms = String::from_utf8_lossy(&output.stdout).trim().to_string();
                println!("   → Socket permissions: {}", perms);
            }
        }
    } else {
        println!("   ✗ Docker socket not found (daemon likely not running)");
    }
    println!();
    Ok(())
}

fn check_user_permissions<E: CommandExecutor>(exec: &E) -> Result<()> {
    println!("[7/8] Checking user Docker group membership...");
    let username = exec.get_username()?;
    println!("   → Current user: {}", username);

    #[cfg(unix)]
    {
        use std::fs;
        if let Ok(group_content) = fs::read_to_string("/etc/group") {
            if let Some(docker_line) = group_content.lines().find(|l| l.starts_with("docker:")) {
                if docker_line.contains(&username) {
                    println!("   ✓ User is in docker group");
                } else {
                    println!("   ✗ User is NOT in docker group");
                    println!("   → Add user with: sudo usermod -aG docker {}", username);
                    println!("   → Then log out and back in, or run: newgrp docker");
                }
            } else {
                println!("   ⚠ docker group not found");
            }
        } else {
            // Fallback to groups command
            let groups_output = exec.execute_simple("groups", &[])?;
            let groups = String::from_utf8_lossy(&groups_output.stdout);
            if groups.contains("docker") {
                println!("   ✓ User is in docker group");
            } else {
                println!("   ✗ User is NOT in docker group");
            }
        }
    }
    println!();
    Ok(())
}

fn check_containerd<E: CommandExecutor>(exec: &E) -> Result<()> {
    println!("[8/8] Checking containerd (Docker runtime)...");
    if exec.check_command_exists("containerd")? {
        println!("   ✓ containerd is installed");

        if exec.check_command_exists("systemctl")? {
            let ctrd_status = exec.execute_simple("systemctl", &["is-active", "containerd"]);
            if let Ok(output) = ctrd_status {
                let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
                match status.as_str() {
                    "active" => println!("   ✓ containerd service is active"),
                    _ => {
                        println!("   ⚠ containerd service is {}", status);
                        println!("   → Start with: sudo systemctl start containerd");
                    }
                }
            }
        }
    } else {
        println!("   ⚠ containerd not found (may be bundled with Docker)");
    }
    println!();
    Ok(())
}

fn print_summary<E: CommandExecutor>(_exec: &E) -> Result<()> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Diagnostic Summary");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Common fixes:");
    println!("  1. Start Docker: sudo systemctl start docker");
    println!("  2. Enable Docker: sudo systemctl enable docker");
    println!("  3. Add user to docker group: sudo usermod -aG docker $USER");
    println!("  4. Check logs: sudo journalctl -xeu docker.service");
    println!("  5. Validate config: sudo python3 -m json.tool /etc/docker/daemon.json");
    println!();
    Ok(())
}

fn check_network_controller_errors<E: CommandExecutor>(exec: &E) -> Result<()> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Network Controller Error Detection");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    if exec.check_command_exists("journalctl")? {
        let log_output = exec.execute_simple(
            "journalctl",
            &["-u", "docker.service", "-n", "50", "--no-pager"],
        );
        if let Ok(output) = log_output {
            let logs = String::from_utf8_lossy(&output.stdout);
            let log_lower = logs.to_lowercase();

            if log_lower.contains("network controller")
                || log_lower.contains("error creating default")
            {
                println!("⚠ Network controller error detected!");
                println!();
                println!("This error typically indicates corrupted Docker network state.");
                println!("Try these fixes (in order):");
                println!();
                println!("1. Clean Docker network state:");
                println!("   sudo rm -rf /var/lib/docker/network");
                println!("   sudo systemctl start docker");
                println!();
                println!("2. If that doesn't work, reset iptables:");
                println!("   sudo iptables -t nat -F");
                println!("   sudo iptables -t mangle -F");
                println!("   sudo iptables -F");
                println!("   sudo iptables -X");
                println!("   sudo systemctl start docker");
                println!();
                println!(
                    "3. As a last resort, clean all Docker data (⚠️  removes all containers/images):"
                );
                println!("   sudo systemctl stop docker");
                println!("   sudo rm -rf /var/lib/docker");
                println!("   sudo systemctl start docker");
                println!();
            }
        }
    }

    Ok(())
}
