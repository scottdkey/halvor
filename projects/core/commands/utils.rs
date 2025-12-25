use crate::config::config_manager;
use crate::utils::update;
use std::env;

/// Check for updates in background (non-blocking)
pub fn check_for_updates() {
    // Check for updates in background (non-blocking)
    if let Ok(Some(new_version)) = update::check_for_updates(env!("CARGO_PKG_VERSION")) {
        if let Ok(true) = update::prompt_for_update(&new_version, env!("CARGO_PKG_VERSION")) {
            if let Err(e) = update::download_and_install_update(&new_version) {
                eprintln!("Failed to install update: {}", e);
            }
        }
    }
}

/// Get version string for CLI
pub fn get_version_string() -> &'static str {
    // Return base version - we'll enhance it in print_version_with_channel
    env!("CARGO_PKG_VERSION")
}

/// Print version with channel information
pub fn print_version_with_channel() {
    let version = env!("CARGO_PKG_VERSION");

    // Try to determine if this is an experimental build
    // by checking if the executable timestamp matches an experimental release
    let version_string = match update::detect_release_channel() {
        Ok(channel) => match channel {
            update::ReleaseChannel::Experimental => format!("{} (experimental)", version),
            update::ReleaseChannel::Stable => format!("{} (stable)", version),
            update::ReleaseChannel::Unknown => {
                // Fall back to configured preference if detection failed
                let configured = config_manager::get_release_channel();
                match configured {
                    config_manager::ReleaseChannel::Experimental => {
                        format!("{} (configured: experimental)", version)
                    }
                    config_manager::ReleaseChannel::Stable => {
                        format!("{} (configured: stable)", version)
                    }
                }
            }
        },
        Err(_) => {
            // If detection completely fails, show configured preference
            let configured = config_manager::get_release_channel();
            match configured {
                config_manager::ReleaseChannel::Experimental => {
                    format!("{} (configured: experimental)", version)
                }
                config_manager::ReleaseChannel::Stable => {
                    format!("{} (configured: stable)", version)
                }
            }
        }
    };

    println!("halvor {}", version_string);
}
