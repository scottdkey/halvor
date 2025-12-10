use crate::{services::host, utils::exec::CommandExecutor};
use anyhow::Result;

/// Handle list command
/// hostname: None = list all hosts, Some(hostname) = list services on that host
pub fn handle_list(hostname: Option<&str>, verbose: bool) -> Result<()> {
    if let Some(hostname) = hostname {
        // List services on a specific host
        list_host_services(hostname, verbose)?;
    } else {
        // List all hosts
        host::list_hosts_display(verbose)?;
    }
    Ok(())
}

/// List services running on a host
fn list_host_services(hostname: &str, verbose: bool) -> Result<()> {
    use crate::config;
    use crate::services::docker;
    use crate::utils::exec::Executor;

    let config = config::load_config()?;
    let exec = Executor::new(hostname, &config)?;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Services on {}", hostname);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Check Docker
    let docker_check =
        exec.execute_simple("docker", &["version", "--format", "{{.Server.Version}}"]);
    if let Ok(output) = docker_check {
        if output.status.success() {
            let version_str = String::from_utf8_lossy(&output.stdout);
            let version = version_str.trim();
            println!("✓ Docker: {}", version);
        } else {
            println!("✗ Docker: Not accessible");
        }
    } else {
        println!("✗ Docker: Not installed");
    }

    // Check Portainer
    if docker::is_container_running(&exec, "portainer")?
        || docker::is_container_running(&exec, "portainer-agent")?
    {
        println!("✓ Portainer: Running");
    } else {
        println!("✗ Portainer: Not running");
    }

    // Check Tailscale
    let tailscale_check = exec.execute_shell("tailscale status --json")?;
    if tailscale_check.status.success() {
        println!("✓ Tailscale: Installed");
    } else {
        println!("✗ Tailscale: Not installed");
    }

    // List all running Docker containers
    println!();
    println!("Running Docker containers:");
    let ps_output = exec.execute_simple(
        "docker",
        &[
            "ps",
            "--format",
            "table {{.Names}}\t{{.Image}}\t{{.Status}}",
        ],
    );
    if let Ok(output) = ps_output {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if !line.trim().is_empty() && !line.contains("NAMES") {
                    println!("  {}", line);
                }
            }
        }
    }

    Ok(())
}
