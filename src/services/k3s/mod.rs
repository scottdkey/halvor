//! K3s cluster management service
//!
//! Handles K3s installation, HA configuration, and etcd snapshots.

// Module declarations
mod cleanup;
mod init;
mod join;
mod kubeconfig;
mod maintenance;
mod status;
mod tools;
mod utils;
mod verify;

// Re-export public utilities
pub use utils::{generate_cluster_token, is_development_mode};

// Re-export tool installation functions
pub use tools::{check_and_install_halvor, check_and_install_helm, check_and_install_kubectl};

// Re-export public functions from modules
pub use init::init_control_plane;
pub use join::join_cluster;
pub use kubeconfig::{fetch_kubeconfig_content, get_kubeconfig, setup_local_kubeconfig};
pub use maintenance::{restore_snapshot, take_snapshot, uninstall};
pub use status::{get_cluster_join_info, show_status};
pub use verify::verify_ha_cluster;
