//! Install Command
//!
//! Unified installer for platform tools and Docker services.
//!
//! Usage:
//!   halvor install <app>              # Install on current system
//!   halvor install <app> -H <host>    # Install on remote host
//!   halvor install --list             # Show all available apps

use halvor_core::config;
use halvor_agent::apps::{AppCategory, find_app, list_apps, k3s};
use halvor_agent::apps::{smb, tailscale};
use halvor_agent::apps::helm_app::HelmApp;
use halvor_agent::apps::{
    nginx_proxy_manager::NginxProxyManager,
    traefik_public::TraefikPublic,
    traefik_private::TraefikPrivate,
    gitea::Gitea,
    pia_vpn::PiaVpn,
    sabnzbd::Sabnzbd,
    qbittorrent::Qbittorrent,
    radarr::Radarr,
    sonarr::Sonarr,
    prowlarr::Prowlarr,
    bazarr::Bazarr,
    smb_storage::SmbStorage,
    halvor_server::HalvorServer,
    portainer_helm::Portainer,
};
use halvor_core::services::helm;
use halvor_docker;
use anyhow::Result;

/// Handle install command
pub fn handle_install(
    hostname: Option<&str>,
    app: Option<&str>,
    list: bool,
    repo: Option<&str>,
    repo_name: Option<&str>,
    name: Option<&str>,
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

    // If --repo is provided, skip app registry and install directly as Helm chart
    if repo.is_some() {
        // External Helm repository - install directly without app registry check
        // Default to frigg (primary control plane) for external Helm charts
        let target_host = if let Some(host) = hostname {
            host
        } else {
            println!("⚠️  No hostname specified for external Helm chart deployment.");
            println!("   Defaulting to 'frigg' (primary cluster node).");
            println!("   Use '-H <hostname>' to specify a different node.\n");
            "frigg"
        };

        println!("Installing from external Helm repository...");
        helm::install_chart(
            target_host,
            app_name,
            None,            // Use chart name as release name
            Some("default"), // Default namespace
            None,            // No values file
            &[],             // No --set flags
            repo,
            repo_name,
            &config,
        )?;
        return Ok(());
    }

    // For Helm charts, default to primary cluster node (frigg) instead of localhost
    // This ensures we deploy to the cluster, not local machine
    let target_host = if let Some(host) = hostname {
        host
    } else {
        // Check if this is a Helm chart deployment
        let is_helm_chart = find_app(app_name)
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

    // Look up the app and use its category
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
            // Get the appropriate HelmApp implementation
            let helm_app: Box<dyn HelmApp> = get_helm_app(app_def.name)?;
            
            // Use custom release name if provided, otherwise use default
            let release_name = name.unwrap_or(helm_app.release_name());
            
            // Use the HelmApp implementation to install
            if repo.is_some() {
                // External repo - use direct helm service
                halvor_core::services::helm::install_chart(
                    target_host,
                    helm_app.chart_name(),
                    Some(release_name),
                    Some(helm_app.namespace()),
                    None,
                    &helm_app.generate_values()?,
                    repo,
                    repo_name,
                    &config,
                )?;
            } else {
                // Use HelmApp trait method with custom release name
                install_helm_app_with_name(&*helm_app, target_host, Some(release_name), &config)?;
            }
        }
    }

    Ok(())
}

/// Install a platform tool (docker, tailscale, smb, k3s, pia-vpn)
fn install_platform_tool(hostname: &str, tool: &str, config: &config::EnvConfig) -> Result<()> {
    match tool {
        "docker" => {
            halvor_docker::install_docker(hostname, config)?;
        }
        "tailscale" => {
            if hostname == "localhost" {
                tailscale::install_tailscale()?;
            } else {
                tailscale::install_tailscale_on_host(hostname, config)?;
            }
        }
        "smb" | "samba" | "cifs" => {
            smb::setup_smb_mounts(hostname, config)?;
        }
        "k3s" | "kubernetes" | "k8s" => {
            // K3s init - initialize primary control plane node
            k3s::init_control_plane(hostname, None, false, config)?;
        }
        "agent" | "halvor-agent" => {
            // Install/update halvor agent
            halvor_agent::install_agent(hostname, config)?;
        }
        "pia-vpn" | "pia" | "vpn" => {
            // PIA VPN is a Helm chart - should be installed via Helm chart category
            anyhow::bail!(
                "PIA VPN is a Helm chart and should be installed on a Kubernetes cluster.\n\
                 Use: halvor install pia-vpn -H <cluster-node>"
            );
        }
        _ => {
            anyhow::bail!("Unknown platform tool: {}", tool);
        }
    }
    Ok(())
}

/// Install a Helm app with an optional custom release name
fn install_helm_app_with_name(
    app: &dyn HelmApp,
    hostname: &str,
    release_name: Option<&str>,
    config: &config::EnvConfig,
) -> Result<()> {
    use halvor_core::services::helm;
    
    let final_release_name = release_name.unwrap_or_else(|| app.release_name());
    
    helm::install_chart(
        hostname,
        app.chart_name(),
        Some(final_release_name),
        Some(app.namespace()),
        None, // No values file - will generate from env vars
        &app.generate_values()?,
        None, // No external repo
        None, // No repo name
        config,
    )
}

/// Get the appropriate HelmApp implementation for an app name
fn get_helm_app(name: &str) -> Result<Box<dyn HelmApp>> {
    match name {
        "nginx-proxy-manager" | "npm" => Ok(Box::new(NginxProxyManager)),
        "portainer" => Ok(Box::new(Portainer)),
        "traefik-public" => Ok(Box::new(TraefikPublic)),
        "traefik-private" => Ok(Box::new(TraefikPrivate)),
        "gitea" => Ok(Box::new(Gitea)),
        "pia-vpn" | "pia" | "vpn" => Ok(Box::new(PiaVpn)),
        "sabnzbd" | "sab" => Ok(Box::new(Sabnzbd)),
        "qbittorrent" | "qbt" | "torrent" => Ok(Box::new(Qbittorrent)),
        "radarr" => Ok(Box::new(Radarr)),
        "sonarr" => Ok(Box::new(Sonarr)),
        "prowlarr" => Ok(Box::new(Prowlarr)),
        "bazarr" => Ok(Box::new(Bazarr)),
        "smb-storage" | "smb" | "storage" => Ok(Box::new(SmbStorage)),
        "halvor-server" | "halvor" | "server" => Ok(Box::new(HalvorServer)),
        _ => {
            // Fall back to generic AppDefinition implementation
            let app_def = find_app(name)
                .ok_or_else(|| anyhow::anyhow!("App '{}' not found", name))?;
            Ok(Box::new(app_def))
        }
    }
}
