// PIA VPN module - organized into submodules for maintainability
mod deploy;
mod verify;
mod vpn_utils;

// Re-export public functions
pub use deploy::deploy_vpn;
pub use verify::verify_vpn;
