use crate::config::{self, EnvConfig, HostConfig};
use crate::exec::CommandExecutor;
use anyhow::{Context, Result};
use std::process::Command;

pub fn install_tailscale() -> Result<()> {
    let os = config::get_os();
    let arch = config::get_arch();

    println!("Installing Tailscale on {} ({})...", os, arch);

    match os {
        "macos" => install_tailscale_macos(),
        "linux" => install_tailscale_linux(),
        "windows" => install_tailscale_windows(),
        _ => {
            anyhow::bail!(
                "Unsupported operating system: {}\nPlease install Tailscale manually from: https://tailscale.com/download",
                os
            );
        }
    }
}

fn install_tailscale_macos() -> Result<()> {
    // Check for Homebrew
    if which::which("brew").is_ok() {
        println!("Detected macOS...");
        println!("Installing via Homebrew...");
        let status = Command::new("brew")
            .args(["install", "tailscale"])
            .status()?;

        if status.success() {
            println!("✓ Tailscale installed via Homebrew");
            println!();
            println!("To start Tailscale, run:");
            println!("  sudo tailscaled");
            println!("  tailscale up");
            Ok(())
        } else {
            anyhow::bail!("Failed to install Tailscale via Homebrew");
        }
    } else {
        anyhow::bail!(
            "Homebrew not found. Please install Homebrew first:\n  /bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
        );
    }
}

fn install_tailscale_linux() -> Result<()> {
    println!("Detected Linux...");

    // Try to detect package manager
    let install_script = if which::which("apt-get").is_ok() {
        println!("Installing via apt (Debian/Ubuntu)...");
        Some("curl -fsSL https://tailscale.com/install.sh | sh")
    } else if which::which("yum").is_ok() {
        println!("Installing via yum (RHEL/CentOS)...");
        Some("curl -fsSL https://tailscale.com/install.sh | sh")
    } else if which::which("dnf").is_ok() {
        println!("Installing via dnf (Fedora)...");
        Some("curl -fsSL https://tailscale.com/install.sh | sh")
    } else {
        None
    };

    if let Some(script) = install_script {
        // Download and execute the install script
        let status = Command::new("sh").arg("-c").arg(script).status()?;

        if status.success() {
            println!("✓ Tailscale installed");
            println!();
            println!("To start Tailscale, run:");
            println!("  sudo tailscale up");
            Ok(())
        } else {
            anyhow::bail!("Failed to install Tailscale");
        }
    } else {
        anyhow::bail!(
            "Unsupported Linux distribution. Please install Tailscale manually:\n  Visit: https://tailscale.com/download"
        );
    }
}

fn install_tailscale_windows() -> Result<()> {
    println!("Detected Windows...");
    println!("Please install Tailscale manually from: https://tailscale.com/download/windows");
    println!();
    println!("Or use winget:");
    println!("  winget install Tailscale.Tailscale");
    Ok(())
}

/// Check if Tailscale is installed and install it if not (for remote execution)
pub fn check_and_install_remote<E: CommandExecutor>(exec: &E) -> Result<()> {
    println!();
    println!("=== Checking Tailscale installation ===");

    if exec.check_command_exists("tailscale")? {
        println!("✓ Tailscale already installed");
        return Ok(());
    }

    println!("Tailscale not found. Installing Tailscale...");

    // Detect package manager
    let pkg_mgr = crate::exec::PackageManager::detect(exec)?;

    match pkg_mgr {
        crate::exec::PackageManager::Apt
        | crate::exec::PackageManager::Yum
        | crate::exec::PackageManager::Dnf => {
            // For Linux, use Tailscale's install script
            println!(
                "Detected {} - using Tailscale install script",
                pkg_mgr.display_name()
            );
            let install_cmd = "curl -fsSL https://tailscale.com/install.sh | sh";
            let output = exec.execute_shell(install_cmd)?;
            if !output.status.success() {
                anyhow::bail!("Failed to install Tailscale");
            }
        }
        crate::exec::PackageManager::Brew => {
            println!(
                "Detected {} - installing via Homebrew",
                pkg_mgr.display_name()
            );
            pkg_mgr.install_package(exec, "tailscale")?;
        }
        crate::exec::PackageManager::Unknown => {
            anyhow::bail!(
                "No supported package manager found. Please install Tailscale manually from: https://tailscale.com/download"
            );
        }
    }

    println!("✓ Tailscale installed");
    println!("Note: Run 'sudo tailscale up' to connect to your tailnet");
    Ok(())
}

/// Get host configuration from config with helpful error message
/// This is used across modules that need to access host configuration
pub fn get_host_config<'a>(config: &'a EnvConfig, hostname: &str) -> Result<&'a HostConfig> {
    config.hosts.get(hostname).with_context(|| {
        format!(
            "Host '{}' not found in .env\n\nAdd configuration to .env:\n  HOST_{}_IP=\"<ip-address>\"\n  HOST_{}_TAILSCALE=\"<tailscale-hostname>\"",
            hostname,
            hostname.to_uppercase(),
            hostname.to_uppercase()
        )
    })
}
