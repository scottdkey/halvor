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
use crate::services::apps::{AppCategory, find_app, list_apps};
use crate::services::helm;
use anyhow::Result;

/// Handle install command
pub fn handle_install(
    hostname: Option<&str>,
    app: Option<&str>,
    list: bool,
    helm: bool,
) -> Result<()> {
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

    let config = config::load_config()?;
    // For Helm charts, default to primary cluster node (frigg) instead of localhost
    // This ensures we deploy to the cluster, not local machine
    let target_host = if let Some(host) = hostname {
        host
    } else {
        // Check if this is a Helm chart deployment
        let is_helm_chart = helm
            || find_app(app_name)
                .map(|app| matches!(app.category, AppCategory::HelmChart))
                .unwrap_or(false);

        if is_helm_chart {
            // Default to frigg (primary control plane) for cluster deployments
            println!("⚠️  No hostname specified for Helm chart deployment.");
            println!("   Defaulting to 'frigg' (primary cluster node).");
            println!("   Use '-H <hostname>' to specify a different node.\n");
            "frigg"
        } else {
            // Platform tools default to localhost
            "localhost"
        }
    };

    // If --helm flag is set, install as Helm chart directly
    if helm {
        // Look up the app to get its namespace
        let namespace = find_app(app_name)
            .and_then(|app| app.namespace)
            .or(Some("default"));
        return helm::install_chart(
            target_host,
            app_name,
            None, // Use chart name as release name
            namespace,
            None, // No values file - will generate from env vars
            &[],  // No --set flags - will generate from env vars
            &config,
        );
    }

    // Otherwise, look up the app and use its category
    let app_def = match find_app(app_name) {
        Some(def) => def,
        None => {
            println!("Unknown app: {}\n", app_name);
            list_apps();
            anyhow::bail!("App '{}' not found. See available apps above.", app_name);
        }
    };

    match app_def.category {
        AppCategory::Platform => {
            install_platform_tool(target_host, app_def.name, &config)?;
        }
        AppCategory::HelmChart => {
            // Get namespace from app definition (defaults to "default" if not specified)
            let namespace = app_def.namespace.unwrap_or("default");
            helm::install_chart(
                target_host,
                app_def.name,
                None, // Use chart name as release name
                Some(namespace),
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
