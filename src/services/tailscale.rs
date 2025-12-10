use crate::config::{self, EnvConfig, HostConfig};
use crate::utils::exec::PackageManager;
use crate::utils::exec::{CommandExecutor, Executor};
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
    let pkg_mgr = PackageManager::detect(exec)?;

    match pkg_mgr {
        PackageManager::Apt | PackageManager::Yum | PackageManager::Dnf => {
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
        PackageManager::Brew => {
            println!(
                "Detected {} - installing via Homebrew",
                pkg_mgr.display_name()
            );
            pkg_mgr.install_package(exec, "tailscale")?;
        }
        PackageManager::Unknown => {
            anyhow::bail!(
                "No supported package manager found. Please install Tailscale manually from: https://tailscale.com/download"
            );
        }
    }

    println!("✓ Tailscale installed");
    println!("Note: Run 'sudo tailscale up' to connect to your tailnet");
    Ok(())
}

/// Install Tailscale on a host (public API for CLI)
/// Works for both local and remote hosts
pub fn install_tailscale_on_host(hostname: &str, config: &EnvConfig) -> Result<()> {
    let exec = Executor::new(hostname, config)?;
    let target_host = exec.target_host(hostname, config)?;
    let is_local = exec.is_local();

    if is_local {
        // For local, use the existing install_tailscale function
        install_tailscale()?;
    } else {
        println!("Installing Tailscale on {} ({})...", hostname, target_host);
        println!();
        check_and_install_remote(&exec)?;
        println!();
        println!("✓ Tailscale installation complete for {}", hostname);
    }

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

#[derive(Debug, Clone)]
pub struct TailscaleDevice {
    pub name: String,
    pub ip: Option<String>,
}

/// List Tailscale devices on the network
pub fn list_tailscale_devices() -> Result<Vec<TailscaleDevice>> {
    let output = Command::new("tailscale")
        .args(&["status", "--json"])
        .output()
        .context("Failed to execute tailscale status")?;

    if !output.status.success() {
        return Ok(Vec::new()); // Tailscale not available or not connected
    }

    let status_json: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("Failed to parse tailscale status JSON")?;

    let mut devices = Vec::new();

    // Parse Tailscale status JSON format
    if let Some(peer_map) = status_json.get("Peer") {
        if let Some(peers) = peer_map.as_object() {
            for (_, peer_data) in peers {
                let name = peer_data
                    .get("DNSName")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                let ip = peer_data
                    .get("TailscaleIPs")
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                devices.push(TailscaleDevice { name, ip });
            }
        }
    }

    Ok(devices)
}

/// Get local Tailscale IP address
pub fn get_tailscale_ip() -> Result<Option<String>> {
    let output = Command::new("tailscale").args(&["ip", "-4"]).output().ok();

    if let Some(output) = output {
        if output.status.success() {
            let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !ip.is_empty() {
                return Ok(Some(ip));
            }
        }
    }

    Ok(None)
}

/// Get local Tailscale hostname
pub fn get_tailscale_hostname() -> Result<Option<String>> {
    let output = Command::new("tailscale")
        .args(&["status", "--json"])
        .output()
        .ok();

    if let Some(output) = output {
        if output.status.success() {
            if let Ok(status_json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                if let Some(dns_name) = status_json.get("Self").and_then(|s| s.get("DNSName")) {
                    if let Some(hostname) = dns_name.as_str() {
                        return Ok(Some(hostname.to_string()));
                    }
                }
            }
        }
    }

    Ok(None)
}
