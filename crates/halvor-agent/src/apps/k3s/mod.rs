//! K3s cluster management service
//!
//! Handles K3s installation, HA configuration, and etcd snapshots.

// Module declarations
pub mod agent_service;
mod cleanup;
mod init;
mod join;
pub mod kubeconfig;
mod maintenance;
mod setup;
mod smb_failover;
mod status;
mod tailscale_config;
mod tools;
mod utils;
mod verify;

// Re-export public functions from modules
pub use agent_service::setup_agent_service;
pub use init::{init_control_plane, prepare_node};
pub use join::join_cluster;
pub use maintenance::regenerate_certificates;
pub use status::{get_cluster_join_info, show_status};
pub use tailscale_config::configure_tailscale_for_k3s;
pub use tools::check_and_install_halvor;
pub use verify::verify_ha_cluster;
