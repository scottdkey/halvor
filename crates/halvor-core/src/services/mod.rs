// Services module - auto-detects and exports all services
// Add new services by creating a file in this directory
// Note: Docker functionality is in halvor-docker crate
// Note: Installable apps (k3s, npm, portainer, smb, tailscale, and all Helm charts) are in halvor-core/apps module
// Note: Helm is a utility service for installing Helm chart apps, not an app itself

pub mod backup;
pub mod helm; // Helm is a utility service, not an app
pub mod host;

// Re-export commonly used service functions
pub use backup::{backup_host, list_backups, restore_host as restore_backup};
pub use helm::{
    install_chart, upgrade_release, uninstall_release, list_releases, list_charts, export_values,
};
pub use host::{
    create_executor, delete_host_config, get_host_config, get_host_config_or_error, get_host_info,
    list_hosts, store_host_config, store_host_info,
};
