//! Apps module - All installable applications
//! This module contains all apps that can be installed via halvor install
//!
//! Note: Helm is a utility for installing Helm chart apps, not an app itself.
//! K3s is an app that must be installed before Helm chart apps can be deployed.
//!
//! Helm chart apps implement the `HelmApp` trait to provide a consistent interface.

pub mod registry;
pub mod helm_app; // Trait for Helm chart apps
pub mod npm; // NPM API functions (not HelmApp)
pub mod nginx_proxy_manager; // Nginx Proxy Manager HelmApp
pub mod portainer;
pub mod smb;
pub mod tailscale;
pub mod k3s;

// Helm app implementations
pub mod traefik_public;
pub mod traefik_private;
pub mod gitea;
pub mod pia_vpn;
pub mod sabnzbd;
pub mod qbittorrent;
pub mod radarr;
pub mod sonarr;
pub mod prowlarr;
pub mod bazarr;
pub mod smb_storage;
pub mod halvor_server;
pub mod portainer_helm;

// Re-export registry
pub use registry::{AppCategory, AppDefinition, APPS, find_app, list_apps};

// Re-export Helm app trait
pub use helm_app::{HelmApp, install_helm_app, upgrade_helm_app, uninstall_helm_app};

// Re-export commonly used app functions
pub use npm::*; // NPM API functions
pub use portainer::{install_agent, install_host};
pub use smb::{setup_smb_mounts, uninstall_smb_mounts};
pub use tailscale::{
    get_tailscale_hostname, get_tailscale_ip, install_tailscale_on_host, list_tailscale_devices,
    show_tailscale_status,
};
pub use k3s::{
    init_control_plane, join_cluster, prepare_node, setup_agent_service, get_cluster_join_info,
    show_status, regenerate_certificates, configure_tailscale_for_k3s, check_and_install_halvor,
    verify_ha_cluster, kubeconfig,
};

