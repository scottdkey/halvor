use crate::config::EnvConfig;
use crate::docker;
use crate::exec::{CommandExecutor, Executor};
use crate::portainer::{PortainerEdition, copy_compose_file, install_agent, install_host};
use crate::tailscale;
use anyhow::{Context, Result};

/// Main entry point for provisioning a host
pub fn provision_host(
    hostname: &str,
    portainer_host: bool,
    portainer_edition: &str,
    config: &EnvConfig,
) -> Result<()> {
    let edition = if portainer_host {
        PortainerEdition::from_str(portainer_edition)
            .with_context(|| format!("Invalid portainer edition: {}", portainer_edition))?
    } else {
        // Edition doesn't matter if not installing host
        PortainerEdition::Ce
    };

    // Create executor - it automatically determines if execution should be local or remote
    let exec = Executor::new(hostname, config)?;

    // Get target host for display and Portainer compose file copying
    let target_host = exec.target_host(hostname, config)?;
    let is_local = exec.is_local();

    if is_local {
        println!("Detected local execution on {}", hostname);
    } else {
        println!("Detected remote execution - provisioning via SSH");
        println!("Provisioning {} ({})...", hostname, target_host);
    }
    println!();

    // Copy Portainer compose file if needed (only for remote)
    if !is_local && portainer_host {
        copy_compose_file(&exec, edition.compose_file())?;
    } else if !is_local {
        copy_compose_file(&exec, "portainer-agent.docker-compose.yml")?;
    }

    // Execute provisioning steps using the executor
    check_sudo_access(&exec, !is_local)?;
    docker::check_and_install(&exec)?;
    tailscale::check_and_install_remote(&exec)?;
    docker::configure_permissions(&exec)?;
    docker::configure_ipv6(&exec)?;

    if portainer_host {
        install_host(&exec, edition)?;
    } else {
        install_agent(&exec)?;
    }

    println!();
    println!("✓ Provisioning complete for {}", hostname);

    Ok(())
}

/// Check sudo access (works for both local and remote)
pub fn check_sudo_access<E: CommandExecutor>(exec: &E, is_remote: bool) -> Result<()> {
    println!("=== Checking sudo access ===");

    if !exec.is_linux()? {
        println!("✓ macOS detected (Docker Desktop handles permissions)");
        return Ok(());
    }

    let output = exec.execute_simple("sudo", &["-n", "true"])?;

    if !output.status.success() {
        println!("Error: Passwordless sudo is required for automated provisioning.");
        println!();
        if is_remote {
            println!("To configure passwordless sudo, run on the target host:");
        } else {
            println!("To configure passwordless sudo, run:");
        }
        println!("  sudo visudo");
        println!();
        println!("Then add this line (replace USERNAME with your username):");
        println!("  USERNAME ALL=(ALL) NOPASSWD: ALL");
        println!();
        if is_remote {
            println!("Or for more security, limit to specific commands:");
            println!(
                "  USERNAME ALL=(ALL) NOPASSWD: /usr/bin/docker, /bin/systemctl, /usr/sbin/usermod, /bin/mkdir, /bin/tee, /bin/cp, /bin/mv, /bin/rm, /usr/bin/python3"
            );
            println!();
        }
        anyhow::bail!("Passwordless sudo not configured");
    }

    println!("✓ Passwordless sudo configured");
    Ok(())
}
