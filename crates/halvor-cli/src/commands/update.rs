//! Update halvor or installed apps

use halvor_core::config;
use halvor_agent::apps::{AppCategory, find_app, k3s};
use halvor_core::services::helm;
use halvor_core::utils::exec::{CommandExecutor, Executor};
use halvor_core::utils::update;
use anyhow::{Context, Result};
use std::env;
use std::io::{self, Write};
use std::process::Command;

/// Handle update command
pub fn handle_update(
    hostname: Option<&str>,
    app: Option<&str>,
    experimental: bool,
    force: bool,
) -> Result<()> {
    let config = config::load_config()?;
    let target_host = hostname.unwrap_or("localhost");

    // Check if we're in development mode from HALVOR_ENV
    let is_dev = env::var("HALVOR_ENV")
        .map(|v| v.to_lowercase() == "development")
        .unwrap_or(false);

    // If app is specified, update that app
    if let Some(app_name) = app {
        // Special case: "halvor" app means update halvor itself
        if app_name == "halvor" {
            return update_halvor_binary(experimental, force, is_dev);
        }
        update_app(target_host, app_name, &config)?;
        return Ok(());
    }

    // If in development mode, just update halvor from local source
    if is_dev {
        return update_halvor_binary(false, false, true);
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

/// Update halvor binary (from GitHub or local source)
fn update_halvor_binary(experimental: bool, force: bool, dev: bool) -> Result<()> {
    let exec = Executor::Local;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Update Halvor");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    if dev {
        println!("Development mode: Building from local source...");
        println!();

        // Stop agent service first
        println!("Stopping agent service...");
        if let Err(e) = halvor_agent::apps::k3s::agent_service::stop_agent_service(&exec) {
            eprintln!("Warning: Failed to stop agent service: {}", e);
        }

        // Find project root (look for Cargo.toml with halvor)
        let project_root = find_project_root()?;
        println!("Project root: {}", project_root.display());

        // Build release binary
        println!();
        println!("Building halvor (release mode)...");
        let status = Command::new("cargo")
            .args(["build", "--release", "--bin", "halvor", "--manifest-path"])
            .arg(project_root.join("crates/halvor-cli/Cargo.toml"))
            .status()
            .context("Failed to run cargo build")?;

        if !status.success() {
            anyhow::bail!("Cargo build failed");
        }

        // Install binary
        println!();
        println!("Installing halvor...");
        let home_dir = std::env::var("HOME")
            .ok()
            .map(std::path::PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("Could not find home directory (HOME not set)"))?;
        let cargo_bin = home_dir.join(".cargo/bin");
        std::fs::create_dir_all(&cargo_bin)?;

        let source = project_root.join("target/release/halvor");
        let dest = cargo_bin.join("halvor");
        std::fs::copy(&source, &dest).with_context(|| {
            format!("Failed to copy {} to {}", source.display(), dest.display())
        })?;

        println!("✓ Installed halvor to {}", dest.display());

        // Restart agent service
        println!();
        println!("Restarting agent service...");
        if let Err(e) = halvor_agent::apps::k3s::agent_service::restart_agent_service(&exec, None) {
            eprintln!("Warning: Failed to restart agent service: {}", e);
            println!("  You can start it manually with: halvor agent start --daemon");
        }

        println!();
        println!("✓ Halvor updated from local source");
    } else {
        // Download from GitHub
        let current_version = env!("CARGO_PKG_VERSION");

        if force {
            if experimental {
                println!("Force mode: Downloading latest experimental version...");
                let latest_version = update::get_latest_experimental_version()?;
                println!("Latest experimental version: {}", latest_version);

                // Stop agent, update, restart
                println!();
                println!("Stopping agent service...");
                if let Err(e) = halvor_agent::apps::k3s::agent_service::stop_agent_service(&exec) {
                    eprintln!("Warning: Failed to stop agent service: {}", e);
                }

                update::download_and_install_update(&latest_version)?;

                println!();
                println!("Restarting agent service...");
                if let Err(e) = halvor_agent::apps::k3s::agent_service::restart_agent_service(&exec, None) {
                    eprintln!("Warning: Failed to restart agent service: {}", e);
                }
            } else {
                println!("Force mode: Downloading latest stable version...");
                let latest_version = update::get_latest_version()?;
                println!("Latest version: {}", latest_version);

                // Stop agent, update, restart
                println!();
                println!("Stopping agent service...");
                if let Err(e) = halvor_agent::apps::k3s::agent_service::stop_agent_service(&exec) {
                    eprintln!("Warning: Failed to stop agent service: {}", e);
                }

                update::download_and_install_update(&latest_version)?;

                println!();
                println!("Restarting agent service...");
                if let Err(e) = halvor_agent::apps::k3s::agent_service::restart_agent_service(&exec, None) {
                    eprintln!("Warning: Failed to restart agent service: {}", e);
                }
            }
        } else if experimental {
            if let Ok(Some(new_version)) = update::check_for_experimental_updates(current_version) {
                if update::prompt_for_update(&new_version, current_version)? {
                    // Stop agent, update, restart
                    println!();
                    println!("Stopping agent service...");
                    if let Err(e) = halvor_agent::apps::k3s::agent_service::stop_agent_service(&exec) {
                        eprintln!("Warning: Failed to stop agent service: {}", e);
                    }

                    update::download_and_install_update(&new_version)?;

                    println!();
                    println!("Restarting agent service...");
                    if let Err(e) = halvor_agent::apps::k3s::agent_service::restart_agent_service(&exec, None) {
                        eprintln!("Warning: Failed to restart agent service: {}", e);
                    }
                }
            } else {
                println!("You're already running the latest experimental version.");
            }
        } else if let Ok(Some(new_version)) = update::check_for_updates(current_version) {
            if update::prompt_for_update(&new_version, current_version)? {
                // Stop agent, update, restart
                println!();
                println!("Stopping agent service...");
                if let Err(e) = halvor_agent::apps::k3s::agent_service::stop_agent_service(&exec) {
                    eprintln!("Warning: Failed to stop agent service: {}", e);
                }

                update::download_and_install_update(&new_version)?;

                println!();
                println!("Restarting agent service...");
                if let Err(e) = halvor_agent::apps::k3s::agent_service::restart_agent_service(&exec, None) {
                    eprintln!("Warning: Failed to restart agent service: {}", e);
                }
            }
        } else {
            println!(
                "You're already running the latest version: {}",
                current_version
            );
        }
    }

    Ok(())
}

/// Find the halvor project root directory
fn find_project_root() -> Result<std::path::PathBuf> {
    // First, check if we're in the project directory
    let current_dir = std::env::current_dir()?;

    // Walk up looking for crates/halvor-cli/Cargo.toml with halvor
    let mut dir = current_dir.as_path();
    loop {
        let cargo_toml = dir.join("crates/halvor-cli/Cargo.toml");
        if cargo_toml.exists() {
            // Verify it's the halvor project
            if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                if content.contains("name = \"halvor\"") {
                    return Ok(dir.to_path_buf());
                }
            }
        }

        // Also check for Cargo.toml at this level
        let cargo_toml = dir.join("Cargo.toml");
        if cargo_toml.exists() {
            if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                if content.contains("name = \"halvor\"") {
                    // This might be the crates/halvor-cli directory, go up one level
                    if let Some(parent) = dir.parent() {
                        if let Some(grandparent) = parent.parent() {
                            let check = grandparent.join("crates/halvor-cli/Cargo.toml");
                            if check.exists() {
                                return Ok(grandparent.to_path_buf());
                            }
                        }
                    }
                    return Ok(dir.to_path_buf());
                }
            }
        }

        if let Some(parent) = dir.parent() {
            dir = parent;
        } else {
            break;
        }
    }

    // Fallback: check common locations
    let home = std::env::var("HOME")
        .ok()
        .map(std::path::PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    let common_paths = [
        home.join("code/halvor"),
        home.join("projects/halvor"),
        home.join("dev/halvor"),
        home.join("src/halvor"),
    ];

    for path in &common_paths {
        let cargo_toml = path.join("crates/halvor-cli/Cargo.toml");
        if cargo_toml.exists() {
            return Ok(path.clone());
        }
    }

    anyhow::bail!(
        "Could not find halvor project root. Make sure you're in the project directory or it's in a common location."
    )
}

/// Update a specific app
fn update_app(hostname: &str, app_name: &str, config: &config::EnvConfig) -> Result<()> {
    let app_def = match find_app(app_name) {
        Some(def) => def,
        None => {
            anyhow::bail!(
                "Unknown app: {}. Use 'halvor install --list' to see available apps.",
                app_name
            );
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
    if exec
        .execute_shell("command -v docker >/dev/null 2>&1")
        .is_ok()
    {
        // Try to update Docker (platform-specific)
        if cfg!(target_os = "linux") {
            let _ = exec.execute_shell("sudo apt-get update && sudo apt-get upgrade -y docker-ce docker-ce-cli containerd.io");
        }
        // On macOS, Docker Desktop handles its own updates
    }

    // Update Tailscale
    println!("  Checking Tailscale...");
    if exec
        .execute_shell("command -v tailscale >/dev/null 2>&1")
        .is_ok()
    {
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
fn update_helm_charts(_hostname: &str, _config: &config::EnvConfig) -> Result<()> {
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
            halvor_core::services::helm::upgrade_release(hostname, release_name, None, &[], config)?;

    Ok(())
}
