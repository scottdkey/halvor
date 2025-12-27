//! App registry and definitions
//!
//! Central registry of all available apps that can be installed via halvor.

/// App category determines how an app is installed
#[derive(Debug, Clone, PartialEq)]
pub enum AppCategory {
    /// Platform tool installed natively (e.g., docker, tailscale)
    Platform,
    /// Kubernetes service deployed via Helm chart
    HelmChart,
}

/// App definition with metadata
#[derive(Debug, Clone)]
pub struct AppDefinition {
    pub name: &'static str,
    pub category: AppCategory,
    pub description: &'static str,
    /// Whether this service requires VPN network
    pub requires_vpn: bool,
    /// Aliases for the app name
    pub aliases: &'static [&'static str],
    /// Kubernetes namespace for Helm charts (None = "default")
    pub namespace: Option<&'static str>,
    /// For Helm chart apps, this is the chart name (same as name for most apps)
    /// This allows the chart name to differ from the app name if needed
    pub helm_chart_name: Option<&'static str>,
}

/// Registry of all available apps
pub static APPS: &[AppDefinition] = &[
    // Platform tools
    AppDefinition {
        name: "docker",
        category: AppCategory::Platform,
        description: "Docker container runtime",
        requires_vpn: false,
        aliases: &[],
        namespace: None, // Not applicable for platform tools
        helm_chart_name: None, // Not applicable for platform tools
    },
    AppDefinition {
        name: "tailscale",
        category: AppCategory::Platform,
        description: "Tailscale VPN client",
        requires_vpn: false,
        aliases: &["ts"],
        namespace: None, // Not applicable for platform tools
        helm_chart_name: None, // Not applicable for platform tools
    },
    AppDefinition {
        name: "smb",
        category: AppCategory::Platform,
        description: "SMB share mounts",
        requires_vpn: false,
        aliases: &["samba", "cifs"],
        namespace: None,
        helm_chart_name: None, // Not applicable for platform tools
    },
    AppDefinition {
        name: "k3s",
        category: AppCategory::Platform,
        description: "K3s Kubernetes cluster (initialize primary node)",
        requires_vpn: false,
        aliases: &["kubernetes", "k8s"],
        namespace: None,
        helm_chart_name: None, // Not applicable for platform tools
    },
    AppDefinition {
        name: "agent",
        category: AppCategory::Platform,
        description: "Halvor agent daemon (for remote execution and automation)",
        requires_vpn: false,
        aliases: &["halvor-agent"],
        namespace: None,
        helm_chart_name: None, // Not applicable for platform tools
    },
    // Helm charts - all implement HelmApp trait
    AppDefinition {
        name: "portainer",
        category: AppCategory::HelmChart,
        description: "Portainer CE/BE/Agent - Container management UI (use deploymentType: ce/be/agent)",
        requires_vpn: false,
        aliases: &[],
        namespace: Some("default"),
        helm_chart_name: Some("portainer"),
    },
    AppDefinition {
        name: "nginx-proxy-manager",
        category: AppCategory::HelmChart,
        description: "Reverse proxy with SSL",
        requires_vpn: false,
        aliases: &["npm", "proxy"],
        namespace: Some("default"),
        helm_chart_name: Some("nginx-proxy-manager"),
    },
    AppDefinition {
        name: "traefik-public",
        category: AppCategory::HelmChart,
        description: "Public Traefik reverse proxy (requires PUBLIC_DOMAIN environment variable)",
        requires_vpn: false,
        aliases: &["traefik-pub", "traefik-dev"],
        namespace: Some("traefik"),
        helm_chart_name: Some("traefik-public"),
    },
    AppDefinition {
        name: "traefik-private",
        category: AppCategory::HelmChart,
        description: "Private Traefik reverse proxy (requires PRIVATE_DOMAIN environment variable, local/Tailnet only)",
        requires_vpn: false,
        aliases: &["traefik-priv", "traefik-me"],
        namespace: Some("traefik"),
        helm_chart_name: Some("traefik-private"),
    },
    AppDefinition {
        name: "gitea",
        category: AppCategory::HelmChart,
        description: "Gitea Git hosting service",
        requires_vpn: false,
        aliases: &["git"],
        namespace: Some("gitea"),
        helm_chart_name: Some("gitea"),
    },
    AppDefinition {
        name: "smb-storage",
        category: AppCategory::HelmChart,
        description: "SMB storage setup for Kubernetes (backups, data, docker-appdata)",
        requires_vpn: false,
        aliases: &["smb", "storage"],
        namespace: Some("kube-system"), // SMB storage needs to be in kube-system for node access
        helm_chart_name: Some("smb-storage"),
    },
    AppDefinition {
        name: "pia-vpn",
        category: AppCategory::HelmChart,
        description: "PIA VPN with HTTP proxy (Kubernetes deployment)",
        requires_vpn: false,
        aliases: &["pia", "vpn"],
        namespace: Some("default"),
        helm_chart_name: Some("pia-vpn"),
    },
    AppDefinition {
        name: "sabnzbd",
        category: AppCategory::HelmChart,
        description: "Usenet download client",
        requires_vpn: true,
        aliases: &["sab"],
        namespace: Some("default"),
        helm_chart_name: Some("sabnzbd"),
    },
    AppDefinition {
        name: "qbittorrent",
        category: AppCategory::HelmChart,
        description: "Torrent download client",
        requires_vpn: true,
        aliases: &["qbt", "torrent"],
        namespace: Some("default"),
        helm_chart_name: Some("qbittorrent"),
    },
    AppDefinition {
        name: "radarr",
        category: AppCategory::HelmChart,
        description: "Movie management and automation",
        requires_vpn: true,
        aliases: &[],
        namespace: Some("default"),
        helm_chart_name: Some("radarr"),
    },
    AppDefinition {
        name: "radarr-4k",
        category: AppCategory::HelmChart,
        description: "Movie management for 4K content",
        requires_vpn: true,
        aliases: &["radarr4k"],
        namespace: Some("default"),
        helm_chart_name: Some("radarr-4k"),
    },
    AppDefinition {
        name: "sonarr",
        category: AppCategory::HelmChart,
        description: "TV show management and automation",
        requires_vpn: true,
        aliases: &[],
        namespace: Some("default"),
        helm_chart_name: Some("sonarr"),
    },
    AppDefinition {
        name: "prowlarr",
        category: AppCategory::HelmChart,
        description: "Indexer manager for *arr apps",
        requires_vpn: true,
        aliases: &[],
        namespace: Some("default"),
        helm_chart_name: Some("prowlarr"),
    },
    AppDefinition {
        name: "bazarr",
        category: AppCategory::HelmChart,
        description: "Subtitle management",
        requires_vpn: true,
        aliases: &[],
        namespace: Some("default"),
        helm_chart_name: Some("bazarr"),
    },
    AppDefinition {
        name: "halvor-server",
        category: AppCategory::HelmChart,
        description: "Halvor server with web UI and agent API",
        requires_vpn: false,
        aliases: &["halvor", "server"],
        namespace: Some("default"),
        helm_chart_name: Some("halvor-server"),
    },
];

/// Find an app by name or alias
pub fn find_app(name: &str) -> Option<&'static AppDefinition> {
    let lower = name.to_lowercase();
    APPS.iter()
        .find(|app| app.name == lower || app.aliases.iter().any(|alias| *alias == lower))
}

/// List all available apps
pub fn list_apps() {
    println!("Available apps:\n");

    println!("Platform Tools:");
    for app in APPS.iter().filter(|a| a.category == AppCategory::Platform) {
        print_app(app);
    }

    println!("\nHelm Charts:");
    for app in APPS.iter().filter(|a| a.category == AppCategory::HelmChart) {
        print_app(app);
    }

    println!("\nUsage:");
    println!("  halvor install <app>                  # Install on current system");
    println!("  halvor install <app> -H <hostname>    # Install on remote host");
    println!("\nNote: Helm charts are automatically detected. No --helm flag needed.");
}

fn print_app(app: &AppDefinition) {
    let aliases = if app.aliases.is_empty() {
        String::new()
    } else {
        format!(" (aliases: {})", app.aliases.join(", "))
    };
    let vpn_note = if app.requires_vpn {
        " [requires vpn]"
    } else {
        ""
    };
    println!(
        "  {:<20} - {}{}{}",
        app.name, app.description, aliases, vpn_note
    );
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
        assert_eq!(sonarr.category, AppCategory::HelmChart);
    }
}
