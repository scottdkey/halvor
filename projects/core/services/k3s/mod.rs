//! K3s cluster management service
//!
//! Handles K3s installation, HA configuration, and etcd snapshots.

// Module declarations
mod agent_service;
mod cleanup;
mod init;
mod join;
mod kubeconfig;
mod maintenance;
mod setup;
mod status;
mod tailscale_config;
mod tools;
mod utils;
mod verify;

// Re-export public functions from modules
pub use init::init_control_plane;
pub use join::join_cluster;
pub use status::{get_cluster_join_info, show_status};
pub use tailscale_config::configure_tailscale_for_k3s;
pub use verify::verify_ha_cluster;
