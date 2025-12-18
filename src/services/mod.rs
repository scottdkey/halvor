// Services module - auto-detects and exports all services
// Add new services by creating a file in this directory

pub mod backup;
pub mod build;
pub mod compose_deployer;
pub mod dev;
pub mod docker;
pub mod host;
pub mod npm;
pub mod pia_vpn;
pub mod portainer;
pub mod provision;
pub mod smb;
pub mod sync;
pub mod tailscale;
pub mod web;

// Re-export commonly used service functions
pub use backup::{backup_host, list_backups, restore_host as restore_backup};
pub use docker::{
    check_and_install as docker_check_and_install,
    configure_permissions as docker_configure_permissions,
};
pub use host::{
    create_executor, delete_host_config, get_host_config, get_host_config_or_error, get_host_info,
    list_hosts, store_host_config, store_host_info,
};
pub use pia_vpn::{build_and_push_vpn_image, deploy_vpn, verify_vpn};
pub use portainer::{install_agent, install_host};
pub use provision::{provision_defaults, provision_guided};
pub use smb::{setup_smb_mounts, uninstall_smb_mounts};
pub use sync::sync_data;
pub use tailscale::{
    get_tailscale_hostname, get_tailscale_ip, install_tailscale_on_host, list_tailscale_devices,
};
