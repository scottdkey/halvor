use crate::config;
use crate::services;
use crate::services::build::cli::build_target;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

/// Handle install command
/// hostname: None = local, Some(hostname) = remote host
pub fn handle_install(
    hostname: Option<&str>,
    service: &str,
    edition: &str,
    host: bool,
) -> Result<()> {
    let config = config::load_config()?;
    let target_host = hostname.unwrap_or("localhost");

    match service.to_lowercase().as_str() {
        "docker" => {
            services::docker::install_docker(target_host, &config)?;
        }
        "tailscale" => {
            if target_host == "localhost" {
                services::tailscale::install_tailscale()?;
            } else {
                services::tailscale::install_tailscale_on_host(target_host, &config)?;
            }
        }
        "portainer" => {
            if host {
                services::portainer::install_portainer_host(target_host, edition, &config)?;
            } else {
                services::portainer::install_portainer_agent(target_host, edition, &config)?;
            }
        }
        "npm" => {
            anyhow::bail!(
                "NPM installation not yet implemented. Use 'halvor {} npm' to configure proxy hosts",
                target_host
            );
        }
        "cli" => {
            install_cli()?;
        }
        _ => {
            anyhow::bail!(
                "Unknown service: {}. Supported services: docker, tailscale, portainer, npm, cli",
                service
            );
        }
    }

    Ok(())
}

/// Build and install CLI to system
fn install_cli() -> Result<()> {
    println!("Building and installing CLI to system...");

    // Determine the current target triple
    let current_target = get_current_target()?;
    println!("Building for target: {}", current_target);

    // Build the CLI for the current platform
    let binary_path = match build_target(&current_target)? {
        Some(path) => {
            println!("✓ Built: {}", path.display());
            path
        }
        None => {
            anyhow::bail!("Failed to build CLI for target: {}", current_target);
        }
    };

    // Install the binary to cargo's bin directory
    println!("Installing CLI to system...");
    let cargo_home = std::env::var("CARGO_HOME")
        .ok()
        .or_else(|| {
            std::env::var("HOME")
                .map(|home| format!("{}/.cargo", home))
                .ok()
        })
        .unwrap_or_else(|| String::from("~/.cargo"));

    let cargo_bin = PathBuf::from(&cargo_home).join("bin");
    std::fs::create_dir_all(&cargo_bin).context("Failed to create cargo bin directory")?;

    let install_path = cargo_bin.join("halvor");

    // Copy the binary to the install location
    std::fs::copy(&binary_path, &install_path).with_context(|| {
        format!(
            "Failed to copy binary from {} to {}",
            binary_path.display(),
            install_path.display()
        )
    })?;

    // Make it executable (if on Unix)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&install_path, std::fs::Permissions::from_mode(0o755))
            .context("Failed to make binary executable")?;
    }

    println!("✓ CLI installed to {}", install_path.display());
    println!("  The 'halvor' command is now available in your PATH");

    Ok(())
}

/// Get the current Rust target triple
fn get_current_target() -> Result<String> {
    // Try to get target from rustc
    let output = Command::new("rustc")
        .args(["-vV"])
        .output()
        .context("Failed to run rustc")?;

    if !output.status.success() {
        anyhow::bail!("rustc command failed");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.starts_with("host: ") {
            return Ok(line[6..].trim().to_string());
        }
    }

    // Fallback: use compile-time target
    if let Ok(target) = std::env::var("TARGET") {
        return Ok(target);
    }

    // Use compile-time target detection
    let default_target: &str = {
        #[cfg(all(target_arch = "x86_64", target_os = "macos"))]
        {
            "x86_64-apple-darwin"
        }
        #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
        {
            "aarch64-apple-darwin"
        }
        #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
        {
            "x86_64-unknown-linux-gnu"
        }
        #[cfg(all(target_arch = "aarch64", target_os = "linux"))]
        {
            "aarch64-unknown-linux-gnu"
        }
        #[cfg(all(target_arch = "x86_64", target_os = "windows"))]
        {
            "x86_64-pc-windows-msvc"
        }
        #[cfg(not(any(
            all(target_arch = "x86_64", target_os = "macos"),
            all(target_arch = "aarch64", target_os = "macos"),
            all(target_arch = "x86_64", target_os = "linux"),
            all(target_arch = "aarch64", target_os = "linux"),
            all(target_arch = "x86_64", target_os = "windows")
        )))]
        {
            anyhow::bail!(
                "Unable to determine current target triple. Please set TARGET environment variable."
            );
        }
    };

    Ok(default_target.to_string())
}
