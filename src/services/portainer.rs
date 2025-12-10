use crate::config::EnvConfig;
use crate::services::docker;
use crate::utils::exec::{CommandExecutor, Executor};
use anyhow::{Context, Result};

/// Portainer edition type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortainerEdition {
    Ce,
    Be,
}

impl PortainerEdition {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "ce" => Ok(PortainerEdition::Ce),
            "be" | "business" | "business-edition" => Ok(PortainerEdition::Be),
            _ => anyhow::bail!("Invalid portainer edition: {}. Must be 'ce' or 'be'", s),
        }
    }

    pub fn compose_file(&self) -> &'static str {
        match self {
            PortainerEdition::Ce => "portainer.docker-compose.yml",
            PortainerEdition::Be => "portainer-be.docker-compose.yml",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            PortainerEdition::Ce => "Community Edition",
            PortainerEdition::Be => "Business Edition",
        }
    }
}

/// Install Portainer host (CE or BE)
pub fn install_host<E: CommandExecutor>(exec: &E, edition: PortainerEdition) -> Result<()> {
    println!();
    println!("=== Installing Portainer {} ===", edition.display_name());

    // Remove existing containers
    println!("Removing any existing Portainer instances...");

    // Check and stop/remove portainer using docker module
    if let Ok(containers) = docker::list_containers(exec) {
        for container in &containers {
            if container == "portainer" || container == "portainer_agent" {
                docker::stop_and_remove_container(exec, container).ok();
            }
        }
    }

    println!("✓ Removed existing Portainer containers");

    // Start Portainer
    exec.mkdir_p("$HOME/portainer")?;

    // Verify compose file exists
    let compose_file_path = "$HOME/portainer/docker-compose.yml";
    if !exec.file_exists(compose_file_path)? {
        anyhow::bail!(
            "Docker compose file not found at {}. Please ensure the compose file has been copied.",
            compose_file_path
        );
    }

    // Get docker compose command from docker module
    let compose_cmd = docker::get_compose_command(exec)?;

    // Use explicit -f flag to specify the compose file with full path
    let compose_file_path = "$HOME/portainer/docker-compose.yml";
    exec.execute_shell_interactive(&format!(
        "cd $HOME/portainer && {} -f {} down 2>/dev/null || true && {} -f {} up -d",
        compose_cmd, compose_file_path, compose_cmd, compose_file_path
    ))?;

    println!(
        "✓ Portainer {} installed and running",
        edition.display_name()
    );
    println!("Access Portainer at https://localhost:9443");
    Ok(())
}

/// Install Portainer Agent
pub fn install_agent<E: CommandExecutor>(exec: &E) -> Result<()> {
    println!();
    println!("=== Installing Portainer Agent ===");

    // Ensure Docker daemon is running
    docker::ensure_docker_running(exec)?;

    // Remove existing containers
    println!("Removing any existing Portainer instances...");

    // Check and stop/remove portainer containers using docker module
    if let Ok(containers) = docker::list_containers(exec) {
        for container in &containers {
            if container == "portainer" || container == "portainer_agent" {
                docker::stop_and_remove_container(exec, container).ok();
            }
        }
    }

    println!("✓ Removed existing Portainer containers");

    // Start Portainer Agent
    exec.mkdir_p("$HOME/portainer")?;

    // Verify compose file exists
    let compose_file_path = "$HOME/portainer/docker-compose.yml";
    if !exec.file_exists(compose_file_path)? {
        anyhow::bail!(
            "Docker compose file not found at {}. Please ensure the compose file has been copied.",
            compose_file_path
        );
    }

    // Get docker compose command from docker module
    let compose_cmd = docker::get_compose_command(exec)?;

    // Use explicit -f flag to specify the compose file with full path
    let compose_file_path = "$HOME/portainer/docker-compose.yml";
    exec.execute_shell_interactive(&format!(
        "cd $HOME/portainer && {} -f {} down 2>/dev/null || true && {} -f {} up -d",
        compose_cmd, compose_file_path, compose_cmd, compose_file_path
    ))?;

    println!("✓ Portainer Agent installed and running");
    println!("Add this agent to your Portainer instance using the agent endpoint");
    Ok(())
}

/// Copy Portainer compose file to remote host
/// This function is used by provision module and expects an Executor
pub fn copy_compose_file<E: CommandExecutor>(exec: &E, compose_filename: &str) -> Result<()> {
    // Find the homelab directory to locate the compose file
    let homelab_dir = crate::config::find_homelab_dir()?;
    let compose_file = homelab_dir.join("compose").join(compose_filename);

    if !compose_file.exists() {
        anyhow::bail!(
            "Portainer docker-compose file not found at {}",
            compose_file.display()
        );
    }

    // Read the compose file
    let compose_content = std::fs::read_to_string(&compose_file)
        .with_context(|| format!("Failed to read compose file: {}", compose_file.display()))?;

    // Create directory first
    exec.mkdir_p("$HOME/portainer")?;

    // Write the file
    exec.write_file(
        "$HOME/portainer/docker-compose.yml",
        compose_content.as_bytes(),
    )
    .with_context(|| {
        format!("Failed to write compose file to $HOME/portainer/docker-compose.yml")
    })?;

    // Verify the file was written correctly
    if !exec.file_exists("$HOME/portainer/docker-compose.yml")? {
        anyhow::bail!(
            "Compose file was not found after writing. Please check permissions and disk space."
        );
    }

    println!("✓ Copied {} to $HOME/portainer/", compose_filename);
    Ok(())
}

/// Install Portainer host on a host (public API for CLI)
pub fn install_portainer_host(hostname: &str, edition: &str, config: &EnvConfig) -> Result<()> {
    let edition_enum = PortainerEdition::from_str(edition)
        .with_context(|| format!("Invalid portainer edition: {}", edition))?;

    let exec = Executor::new(hostname, config)?;
    let target_host = exec.target_host(hostname, config)?;
    let is_local = exec.is_local();

    if is_local {
        println!(
            "Installing Portainer {} locally on {}...",
            edition_enum.display_name(),
            hostname
        );
    } else {
        println!(
            "Installing Portainer {} on {} ({})...",
            edition_enum.display_name(),
            hostname,
            target_host
        );
    }

    // Copy compose file (needed for both local and remote)
    copy_compose_file(&exec, edition_enum.compose_file())?;
    println!();

    install_host(&exec, edition_enum)?;

    println!();
    println!(
        "✓ Portainer {} installation complete for {}",
        edition_enum.display_name(),
        hostname
    );

    Ok(())
}

/// Install Portainer Agent on a host (public API for CLI)
pub fn install_portainer_agent(hostname: &str, edition: &str, config: &EnvConfig) -> Result<()> {
    // For agent, edition is currently not used (agent doesn't have CE/BE distinction in the same way)
    // But we accept it for consistency and future use
    let _edition_enum = PortainerEdition::from_str(edition)
        .with_context(|| format!("Invalid portainer edition: {}", edition))?;

    let exec = Executor::new(hostname, config)?;
    let target_host = exec.target_host(hostname, config)?;
    let is_local = exec.is_local();

    if is_local {
        println!("Installing Portainer Agent locally on {}...", hostname);
    } else {
        println!(
            "Installing Portainer Agent on {} ({})...",
            hostname, target_host
        );
    }

    // Copy compose file (needed for both local and remote)
    copy_compose_file(&exec, "portainer-agent.docker-compose.yml")?;
    println!();

    install_agent(&exec)?;

    println!();
    println!("✓ Portainer Agent installation complete for {}", hostname);

    Ok(())
}
