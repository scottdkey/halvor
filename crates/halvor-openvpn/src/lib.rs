// Halvor OpenVPN Library
// Container entrypoint and deployment/service logic for PIA VPN

// Container entrypoint modules
pub mod config;
pub mod download;
pub mod logs;
pub mod process;
pub mod test;

// Service modules (deployment, verification)
pub mod deploy;
pub mod verify;
pub mod vpn_utils;

// Re-export service functions
pub use deploy::deploy_vpn;
pub use verify::verify_vpn;

