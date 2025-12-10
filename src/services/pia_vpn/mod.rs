// PIA VPN module - organized into submodules for maintainability
mod build;
mod deploy;
mod verify;
mod vpn_utils;

// Re-export public functions
pub use build::build_and_push_vpn_image;
pub use deploy::deploy_vpn;
pub use verify::verify_vpn;
