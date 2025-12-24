//! Install Command
//!
//! Unified installer for platform tools and Docker services.
//!
//! Usage:
//!   halvor install <app>              # Install on current system
//!   halvor install <app> -H <host>    # Install on remote host
//!   halvor install --list             # Show all available apps

use crate::config;
use crate::services;
use crate::services::compose_deployer::{AppCategory, find_app, list_apps};
use crate::services::helm;
use anyhow::Result;

/// Handle install command
pub fn handle_install(hostname: Option<&str>, app: Option<&str>, list: bool) -> Result<()> {
    // Handle --list flag
    if list {
        list_apps();
        return Ok(());
    }

    // Require app name
    let app_name = match app {
        Some(name) => name,
        None => {
            list_apps();
            println!("\nError: No app specified. Use 'halvor install <app>' to install.");
            return Ok(());
        }
    };

    // Look up the app
    let app_def = match find_app(app_name) {
        Some(def) => def,
        None => {
            println!("Unknown app: {}\n", app_name);
            list_apps();
            anyhow::bail!("App '{}' not found. See available apps above.", app_name);
        }
    };

    let config = config::load_config()?;
    let target_host = hostname.unwrap_or("localhost");

    match app_def.category {
        AppCategory::Platform => {
            install_platform_tool(target_host, app_def.name, &config)?;
        }
        AppCategory::DockerService => {
            services::compose_deployer::deploy_compose_service(target_host, app_def, &config)?;
        }
        AppCategory::HelmChart => {
            // Determine namespace based on chart
            let namespace = match app_def.name {
                "traefik-public" | "traefik-private" => Some("traefik"),
                "gitea" => Some("gitea"),
                "smb-storage" => Some("kube-system"), // SMB storage needs to be in kube-system for node access
                _ => Some("default"),
            };
            helm::install_chart(
                target_host,
                app_def.name,
                None, // Use chart name as release name
                namespace.as_deref(),
                None, // No values file - will generate from env vars
                &[],  // No --set flags - will generate from env vars
                &config,
            )?;
        }
    }

    Ok(())
}

/// Install a platform tool (docker, tailscale)
fn install_platform_tool(hostname: &str, tool: &str, config: &config::EnvConfig) -> Result<()> {
    match tool {
        "docker" => {
            services::docker::install_docker(hostname, config)?;
        }
        "tailscale" => {
            if hostname == "localhost" {
                services::tailscale::install_tailscale()?;
            } else {
                services::tailscale::install_tailscale_on_host(hostname, config)?;
            }
        }
        _ => {
            anyhow::bail!("Unknown platform tool: {}", tool);
        }
    }
    Ok(())
}
