use crate::docker;
use crate::exec::CommandExecutor;
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
                exec.execute_simple("docker", &["stop", container]).ok();
                exec.execute_simple("docker", &["rm", container]).ok();
            }
        }
    }

    println!("✓ Removed existing Portainer containers");

    // Start Portainer
    exec.mkdir_p("$HOME/portainer")?;

    // Try docker compose, fallback to docker-compose
    let compose_cmd = if exec.check_command_exists("docker")? {
        "docker compose"
    } else {
        "docker-compose"
    };

    exec.execute_shell_interactive(&format!(
        "cd $HOME/portainer && {} down 2>/dev/null || true && {} up -d",
        compose_cmd, compose_cmd
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

    // Remove existing containers
    println!("Removing any existing Portainer instances...");

    // Check and stop/remove portainer containers using docker module
    if let Ok(containers) = docker::list_containers(exec) {
        for container in &containers {
            if container == "portainer" || container == "portainer_agent" {
                exec.execute_simple("docker", &["stop", container]).ok();
                exec.execute_simple("docker", &["rm", container]).ok();
            }
        }
    }

    println!("✓ Removed existing Portainer containers");

    // Start Portainer Agent
    exec.mkdir_p("$HOME/portainer")?;

    // Try docker compose, fallback to docker-compose
    let compose_cmd = if exec.check_command_exists("docker")? {
        "docker compose"
    } else {
        "docker-compose"
    };

    exec.execute_shell_interactive(&format!(
        "cd $HOME/portainer && {} down 2>/dev/null || true && {} up -d",
        compose_cmd, compose_cmd
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
    )?;

    println!("✓ Copied {} to remote system", compose_filename);
    Ok(())
}
