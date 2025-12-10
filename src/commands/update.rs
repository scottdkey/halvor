use crate::utils::update;
use anyhow::Result;
use std::env;

pub fn handle_update(experimental: bool, force: bool) -> Result<()> {
    let current_version = env!("CARGO_PKG_VERSION");

    if force {
        // Force mode: get the latest version and install it regardless of current version
        if experimental {
            println!("Force mode: Downloading latest experimental version...");
            let latest_version = update::get_latest_experimental_version()?;
            println!("Latest experimental version: {}", latest_version);
            update::download_and_install_update(&latest_version)?;
        } else {
            println!("Force mode: Downloading latest stable version...");
            let latest_version = update::get_latest_version()?;
            println!("Latest version: {}", latest_version);
            update::download_and_install_update(&latest_version)?;
        }
    } else if experimental {
        // Experimental channel: check for updates based on timestamps (version less)
        if let Ok(Some(new_version)) = update::check_for_experimental_updates(current_version) {
            if update::prompt_for_update(&new_version, current_version)? {
                update::download_and_install_update(&new_version)?;
            }
        } else {
            println!("You're already running the latest experimental version.");
        }
    } else if let Ok(Some(new_version)) = update::check_for_updates(current_version) {
        if update::prompt_for_update(&new_version, current_version)? {
            update::download_and_install_update(&new_version)?;
        }
    } else {
        println!(
            "You're already running the latest version: {}",
            current_version
        );
    }
    Ok(())
}
