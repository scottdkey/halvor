//! Compose Deployer Service
//!
//! Deploys Docker Compose services from the compose/ directory to target hosts.
//! Handles network dependencies and environment setup.
//!
//! Environment variables:
//! - HALVOR_ENV: Set to "development" for dev mode (builds locally, runs from repo)
//! - COMPOSE_DEPLOY_PATH: Base path for production deployments (default: $HOME)

use crate::config::EnvConfig;
use crate::services::docker;
use crate::utils::exec::{CommandExecutor, Executor};
use anyhow::{Context, Result};
use std::path::Path;

/// Check if we're in development mode
fn is_development_mode() -> bool {
    std::env::var("HALVOR_ENV")
        .map(|v| v.to_lowercase() == "development")
        .unwrap_or(false)
}

/// Get the base path for deployments (production only)
fn get_deploy_base_path() -> String {
    std::env::var("COMPOSE_DEPLOY_PATH").unwrap_or_else(|_| "$HOME".to_string())
}

/// App category determines how an app is installed
#[derive(Debug, Clone, PartialEq)]
pub enum AppCategory {
    /// Platform tool installed natively (e.g., docker, tailscale)
    Platform,
    /// Docker service deployed via compose file
    DockerService,
}

/// App definition with metadata
#[derive(Debug, Clone)]
pub struct AppDefinition {
    pub name: &'static str,
    pub category: AppCategory,
    pub description: &'static str,
    /// For DockerService: name of the compose directory
    pub compose_dir: Option<&'static str>,
    /// Whether this service requires vpn_network
    pub requires_vpn: bool,
    /// Aliases for the app name
    pub aliases: &'static [&'static str],
}

/// Registry of all available apps
pub static APPS: &[AppDefinition] = &[
    // Platform tools
    AppDefinition {
        name: "docker",
        category: AppCategory::Platform,
        description: "Docker container runtime",
        compose_dir: None,
        requires_vpn: false,
        aliases: &[],
    },
    AppDefinition {
        name: "tailscale",
        category: AppCategory::Platform,
        description: "Tailscale VPN client",
        compose_dir: None,
        requires_vpn: false,
        aliases: &["ts"],
    },
    // Docker services
    AppDefinition {
        name: "portainer",
        category: AppCategory::DockerService,
        description: "Container management UI",
        compose_dir: Some("portainer"),
        requires_vpn: false,
        aliases: &[],
    },
    AppDefinition {
        name: "portainer-agent",
        category: AppCategory::DockerService,
        description: "Portainer agent for remote management",
        compose_dir: Some("portainer-agent"),
        requires_vpn: false,
        aliases: &["agent"],
    },
    AppDefinition {
        name: "pia-vpn",
        category: AppCategory::DockerService,
        description: "PIA VPN with HTTP proxy",
        compose_dir: Some("vpn"),
        requires_vpn: false,
        aliases: &["pia", "vpn"],
    },
    AppDefinition {
        name: "nginx-proxy-manager",
        category: AppCategory::DockerService,
        description: "Reverse proxy with SSL",
        compose_dir: Some("nginx-proxy-manager"),
        requires_vpn: false,
        aliases: &["npm", "proxy"],
    },
    AppDefinition {
        name: "sabnzbd",
        category: AppCategory::DockerService,
        description: "Usenet download client",
        compose_dir: Some("sabnzbd"),
        requires_vpn: true,
        aliases: &["sab"],
    },
    AppDefinition {
        name: "qbittorrent",
        category: AppCategory::DockerService,
        description: "Torrent download client",
        compose_dir: Some("qbittorrent"),
        requires_vpn: true,
        aliases: &["qbt", "torrent"],
    },
    AppDefinition {
        name: "radarr",
        category: AppCategory::DockerService,
        description: "Movie management and automation",
        compose_dir: Some("radarr"),
        requires_vpn: true,
        aliases: &[],
    },
    AppDefinition {
        name: "radarr-4k",
        category: AppCategory::DockerService,
        description: "Movie management for 4K content",
        compose_dir: Some("radarr-4k"),
        requires_vpn: true,
        aliases: &["radarr4k"],
    },
    AppDefinition {
        name: "sonarr",
        category: AppCategory::DockerService,
        description: "TV show management and automation",
        compose_dir: Some("sonarr"),
        requires_vpn: true,
        aliases: &[],
    },
    AppDefinition {
        name: "prowlarr",
        category: AppCategory::DockerService,
        description: "Indexer manager for *arr apps",
        compose_dir: Some("prowlarr"),
        requires_vpn: true,
        aliases: &[],
    },
    AppDefinition {
        name: "bazarr",
        category: AppCategory::DockerService,
        description: "Subtitle management",
        compose_dir: Some("bazarr"),
        requires_vpn: true,
        aliases: &[],
    },
];

/// Find an app by name or alias
pub fn find_app(name: &str) -> Option<&'static AppDefinition> {
    let lower = name.to_lowercase();
    APPS.iter().find(|app| {
        app.name == lower || app.aliases.iter().any(|alias| *alias == lower)
    })
}

/// List all available apps
pub fn list_apps() {
    println!("Available apps:\n");

    println!("Platform Tools:");
    for app in APPS.iter().filter(|a| a.category == AppCategory::Platform) {
        print_app(app);
    }

    println!("\nDocker Services:");
    for app in APPS.iter().filter(|a| a.category == AppCategory::DockerService) {
        print_app(app);
    }

    println!("\nUsage:");
    println!("  halvor install <app>                  # Install on current system");
    println!("  halvor install <app> -H <hostname>    # Install on remote host");
}

fn print_app(app: &AppDefinition) {
    let aliases = if app.aliases.is_empty() {
        String::new()
    } else {
        format!(" (aliases: {})", app.aliases.join(", "))
    };
    let vpn_note = if app.requires_vpn { " [requires vpn]" } else { "" };
    println!("  {:<20} - {}{}{}", app.name, app.description, aliases, vpn_note);
}

/// Deploy a Docker Compose service to a target host
pub fn deploy_compose_service(
    hostname: &str,
    app: &AppDefinition,
    config: &EnvConfig,
) -> Result<()> {
    let compose_dir = app.compose_dir.context("App does not have a compose directory")?;

    let is_dev = is_development_mode();
    let mode_str = if is_dev { "development" } else { "production" };
    println!("Deploying {} to {} ({} mode)...", app.name, hostname, mode_str);

    // Create executor for target host
    let exec = Executor::new(hostname, config)?;

    // Check if docker is available
    if docker::get_compose_command(&exec).is_err() {
        anyhow::bail!(
            "Docker is not installed on {}. Run 'halvor install docker -H {}' first.",
            hostname,
            hostname
        );
    }

    // If service requires VPN, check that vpn_network exists
    if app.requires_vpn {
        ensure_vpn_network(&exec)?;
    }

    let compose_cmd = docker::get_compose_command(&exec)?;

    if is_dev {
        // Development mode: run directly from repo's compose directory
        deploy_development_mode(&exec, app, compose_dir, &compose_cmd)?;
    } else {
        // Production mode: copy files to target directory and run
        deploy_production_mode(&exec, app, compose_dir, &compose_cmd)?;
    }

    println!("✓ {} deployed successfully", app.name);

    Ok(())
}

/// Deploy in development mode - build and run from repo directory
fn deploy_development_mode<E: CommandExecutor>(
    exec: &E,
    app: &AppDefinition,
    compose_dir: &str,
    compose_cmd: &str,
) -> Result<()> {
    let compose_path = find_compose_path(compose_dir)?;
    let compose_path_str = compose_path
        .canonicalize()
        .with_context(|| format!("Failed to resolve compose path: {}", compose_path.display()))?;
    let compose_path_str = compose_path_str.to_string_lossy();

    println!("  Using compose directory: {}", compose_path_str);

    // Stop existing container if running
    println!("  Stopping existing container...");
    exec.execute_shell_interactive(&format!(
        "cd \"{}\" && {} down 2>/dev/null || true",
        compose_path_str, compose_cmd
    ))?;

    // Build the container locally
    println!("  Building {} container...", app.name);
    exec.execute_shell_interactive(&format!(
        "cd \"{}\" && {} build",
        compose_path_str, compose_cmd
    ))?;

    // Start the service
    println!("  Starting {}...", app.name);
    exec.execute_shell_interactive(&format!(
        "cd \"{}\" && {} up -d",
        compose_path_str, compose_cmd
    ))?;

    Ok(())
}

/// Deploy in production mode - copy files to target and run
fn deploy_production_mode<E: CommandExecutor>(
    exec: &E,
    app: &AppDefinition,
    compose_dir: &str,
    compose_cmd: &str,
) -> Result<()> {
    // Get the compose file content from the local compose directory
    let compose_content = get_compose_file_content(compose_dir)?;

    // Create target directory
    let base_path = get_deploy_base_path();
    let target_dir = format!("{}/{}", base_path, app.name);
    exec.mkdir_p(&target_dir)?;

    // Write compose file to target
    let target_compose = format!("{}/docker-compose.yml", target_dir);
    exec.write_file(&target_compose, compose_content.as_bytes())?;

    // Copy .env.example if it exists and .env doesn't
    if let Ok(env_example) = get_env_example_content(compose_dir) {
        let target_env = format!("{}/.env", target_dir);
        if !exec.file_exists(&target_env).unwrap_or(false) {
            println!("  Creating .env from example...");
            exec.write_file(&target_env, env_example.as_bytes())?;
            println!("  Note: Edit {}/.env to configure the service", app.name);
        }
    }

    // Deploy with docker compose
    println!("  Starting {}...", app.name);

    exec.execute_shell_interactive(&format!(
        "cd {} && {} down 2>/dev/null || true && {} up -d",
        target_dir, compose_cmd, compose_cmd
    ))?;

    Ok(())
}

/// Ensure vpn_network exists (required by media services)
fn ensure_vpn_network<E: CommandExecutor>(exec: &E) -> Result<()> {
    // Check if vpn_network exists
    let result = exec.execute_shell("docker network inspect vpn_network 2>/dev/null");

    if result.is_err() || !result.unwrap().status.success() {
        println!("  vpn_network not found. Creating...");
        println!("  Note: For full VPN protection, install VPN first: halvor install vpn");

        // Create the network with the expected configuration
        exec.execute_shell(
            "docker network create --driver bridge --subnet 172.20.0.0/16 vpn_network",
        )?;

        println!("  Created vpn_network (172.20.0.0/16)");
    }

    Ok(())
}

/// Get compose file content from local compose directory
fn get_compose_file_content(compose_dir: &str) -> Result<String> {
    // Find the compose directory relative to the project root
    let compose_path = find_compose_path(compose_dir)?;
    let compose_file = compose_path.join("docker-compose.yml");

    std::fs::read_to_string(&compose_file).with_context(|| {
        format!(
            "Failed to read compose file: {}",
            compose_file.display()
        )
    })
}

/// Get .env.example content if it exists
fn get_env_example_content(compose_dir: &str) -> Result<String> {
    let compose_path = find_compose_path(compose_dir)?;
    let env_file = compose_path.join(".env.example");

    std::fs::read_to_string(&env_file).with_context(|| {
        format!("Failed to read .env.example: {}", env_file.display())
    })
}

/// Find the compose directory path
fn find_compose_path(compose_dir: &str) -> Result<std::path::PathBuf> {
    // Try relative to current directory first
    let relative = Path::new("compose").join(compose_dir);
    if relative.exists() {
        return Ok(relative);
    }

    // Try relative to executable
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            // Check if we're in a development environment
            let dev_path = exe_dir
                .parent() // target
                .and_then(|p| p.parent()) // target/debug or target/release
                .map(|p| p.join("compose").join(compose_dir));

            if let Some(path) = dev_path {
                if path.exists() {
                    return Ok(path);
                }
            }
        }
    }

    // Try from CARGO_MANIFEST_DIR (development)
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let path = Path::new(&manifest_dir).join("compose").join(compose_dir);
        if path.exists() {
            return Ok(path);
        }
    }

    anyhow::bail!(
        "Could not find compose directory for '{}'. Make sure compose/{} exists.",
        compose_dir,
        compose_dir
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_app_by_name() {
        assert!(find_app("docker").is_some());
        assert!(find_app("sonarr").is_some());
        assert!(find_app("unknown").is_none());
    }

    #[test]
    fn test_find_app_by_alias() {
        let app = find_app("npm").unwrap();
        assert_eq!(app.name, "nginx-proxy-manager");

        let app = find_app("ts").unwrap();
        assert_eq!(app.name, "tailscale");
    }

    #[test]
    fn test_app_categories() {
        let docker = find_app("docker").unwrap();
        assert_eq!(docker.category, AppCategory::Platform);

        let sonarr = find_app("sonarr").unwrap();
        assert_eq!(sonarr.category, AppCategory::DockerService);
    }
}
