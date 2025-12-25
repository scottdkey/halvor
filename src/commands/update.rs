//! Update halvor or installed apps

use crate::config;
use crate::services::apps::{find_app, AppCategory};
use crate::services::helm;
use crate::utils::exec::{CommandExecutor, Executor};
use crate::utils::update;
use anyhow::Result;
use std::env;
use std::io::{self, Write};

/// Handle update command
pub fn handle_update(
    hostname: Option<&str>,
    app: Option<&str>,
    experimental: bool,
    force: bool,
) -> Result<()> {
    let config = config::load_config()?;
    let target_host = hostname.unwrap_or("localhost");

    // If app is specified, update that app
    if let Some(app_name) = app {
        update_app(target_host, app_name, &config)?;
        return Ok(());
    }

    // Otherwise, update halvor itself and prompt for updating everything
    let current_version = env!("CARGO_PKG_VERSION");

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Update System");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // First, check for halvor updates
    println!("Checking for halvor updates...");
    let halvor_updated = if force {
        if experimental {
            println!("Force mode: Downloading latest experimental version...");
            let latest_version = update::get_latest_experimental_version()?;
            println!("Latest experimental version: {}", latest_version);
            update::download_and_install_update(&latest_version)?;
            true
        } else {
            println!("Force mode: Downloading latest stable version...");
            let latest_version = update::get_latest_version()?;
            println!("Latest version: {}", latest_version);
            update::download_and_install_update(&latest_version)?;
            true
        }
    } else if experimental {
        if let Ok(Some(new_version)) = update::check_for_experimental_updates(current_version) {
            if update::prompt_for_update(&new_version, current_version)? {
                update::download_and_install_update(&new_version)?;
                true
            } else {
                false
            }
        } else {
            println!("You're already running the latest experimental version.");
            false
        }
    } else if let Ok(Some(new_version)) = update::check_for_updates(current_version) {
        if update::prompt_for_update(&new_version, current_version)? {
            update::download_and_install_update(&new_version)?;
            true
        } else {
            false
        }
    } else {
        println!(
            "You're already running the latest version: {}",
            current_version
        );
        false
    };

    if halvor_updated {
        println!();
        println!("⚠️  halvor was updated. Please restart the CLI to use the new version.");
        println!("   Continuing with app updates...");
        println!();
    }

    // Prompt to update all apps
    println!("Update all installed apps on {}?", target_host);
    println!("⚠️  WARNING: This may update system packages and Docker containers.");
    println!("   Some updates may require service restarts.");
    println!();
    print!("Continue? [y/N]: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let response = input.trim().to_lowercase();
    if response != "y" && response != "yes" {
        println!("Update cancelled.");
        return Ok(());
    }
    println!();

    // Update platform tools
    update_platform_tools(target_host, &config)?;

    // Update Helm charts
    update_helm_charts(target_host, &config)?;

    println!();
    println!("✓ Update complete");

    Ok(())
}

/// Update a specific app
fn update_app(hostname: &str, app_name: &str, config: &config::EnvConfig) -> Result<()> {
    let app_def = match find_app(app_name) {
        Some(def) => def,
        None => {
            anyhow::bail!("Unknown app: {}. Use 'halvor install --list' to see available apps.", app_name);
        }
    };

    match app_def.category {
        AppCategory::Platform => {
            update_platform_tool(hostname, app_def.name, config)?;
        }
        AppCategory::HelmChart => {
            update_helm_chart(hostname, app_def.name, config)?;
        }
    }

    Ok(())
}

/// Update platform tools
fn update_platform_tools(hostname: &str, config: &config::EnvConfig) -> Result<()> {
    let exec = Executor::new(hostname, config)?;

    println!("Updating platform tools...");

    // Update Docker
    println!("  Checking Docker...");
    if exec.execute_shell("command -v docker >/dev/null 2>&1").is_ok() {
        // Try to update Docker (platform-specific)
        if cfg!(target_os = "linux") {
            let _ = exec.execute_shell("sudo apt-get update && sudo apt-get upgrade -y docker-ce docker-ce-cli containerd.io");
        }
        // On macOS, Docker Desktop handles its own updates
    }

    // Update Tailscale
    println!("  Checking Tailscale...");
    if exec.execute_shell("command -v tailscale >/dev/null 2>&1").is_ok() {
        if cfg!(target_os = "linux") {
            let _ = exec.execute_shell("sudo tailscale update");
        }
        // On macOS, Tailscale handles its own updates
    }

    // K3s updates
    println!("  Checking K3s...");
    if exec.execute_shell("command -v k3s >/dev/null 2>&1").is_ok() {
        println!("    Use 'k3s upgrade' to upgrade K3s cluster");
    }

    Ok(())
}

/// Update a specific platform tool
fn update_platform_tool(hostname: &str, tool: &str, config: &config::EnvConfig) -> Result<()> {
    let exec = Executor::new(hostname, config)?;

    match tool {
        "docker" => {
            println!("Updating Docker on {}...", hostname);
            if cfg!(target_os = "linux") {
                exec.execute_shell("sudo apt-get update && sudo apt-get upgrade -y docker-ce docker-ce-cli containerd.io")?;
            } else {
                println!("Docker Desktop on macOS/Windows handles its own updates.");
            }
        }
        "tailscale" => {
            println!("Updating Tailscale on {}...", hostname);
            if cfg!(target_os = "linux") {
                exec.execute_shell("sudo tailscale update")?;
            } else {
                println!("Tailscale on macOS/Windows handles its own updates.");
            }
        }
        "k3s" | "kubernetes" | "k8s" => {
            println!("Updating K3s on {}...", hostname);
            println!("⚠️  Use 'k3s upgrade' command to upgrade K3s cluster.");
            println!("   Or reinstall K3s to get the latest version.");
        }
        "smb" | "samba" | "cifs" => {
            println!("SMB mounts don't require updates.");
        }
        _ => {
            anyhow::bail!("Unknown platform tool: {}", tool);
        }
    }

    Ok(())
}

/// Update Helm charts
fn update_helm_charts(hostname: &str, config: &config::EnvConfig) -> Result<()> {
    println!("Updating Helm charts...");
    // Get list of installed Helm releases and update them
    // This is a simplified version - in practice, you'd want to list all releases
    println!("  Use 'halvor update <app>' to update specific Helm charts.");
    Ok(())
}

/// Update a specific Helm chart
fn update_helm_chart(hostname: &str, chart_name: &str, config: &config::EnvConfig) -> Result<()> {
    println!("Updating Helm chart '{}' on {}...", chart_name, hostname);
    
    let release_name = chart_name; // Use chart name as release name
    
    // Use helm upgrade_release (it will detect namespace from the release)
    helm::upgrade_release(hostname, release_name, None, &[], config)?;
    
    Ok(())
}
