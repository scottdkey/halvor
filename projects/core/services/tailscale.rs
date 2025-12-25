use crate::config::{self, EnvConfig, HostConfig};
use crate::utils::exec::PackageManager;
use crate::utils::exec::{CommandExecutor, Executor};
use anyhow::{Context, Result};
use reqwest;
use std::process::Command;

/// Check if Tailscale is installed
pub fn is_tailscale_installed<E: CommandExecutor>(exec: &E) -> bool {
    exec.check_command_exists("tailscale").unwrap_or(false)
}

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

            // Download install script using native Rust HTTP client
            println!("Downloading Tailscale install script...");
            let script_url = "https://tailscale.com/install.sh";
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .context("Failed to create HTTP client")?;

            let script_content = client
                .get(script_url)
                .send()
                .context("Failed to download Tailscale install script")?
                .error_for_status()
                .context("HTTP error downloading Tailscale install script")?
                .text()
                .context("Failed to read Tailscale install script content")?;

            // Write script to remote host and execute
            let remote_script_path = "/tmp/tailscale-install.sh";
            exec.write_file(remote_script_path, script_content.as_bytes())
                .context("Failed to write Tailscale install script to remote host")?;

            exec.execute_shell(&format!("chmod +x {}", remote_script_path))
                .context("Failed to make Tailscale install script executable")?;

            let output = exec.execute_shell(&format!("sh {}", remote_script_path))?;
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
    // Try normalized hostname lookup
    let actual_hostname = crate::config::service::find_hostname_in_config(hostname, config)
        .unwrap_or_else(|| hostname.to_string());
    config.hosts.get(&actual_hostname).with_context(|| {
        format!(
            "Host '{}' not found in .env\n\nAdd configuration to .env:\n  HOST_{}_IP=\"<ip-address>\"\n  HOST_{}_HOSTNAME=\"<hostname>\"",
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

/// Get Tailscale IP on a remote host via executor
pub fn get_tailscale_ip_remote<E: CommandExecutor>(exec: &E) -> Result<Option<String>> {
    // Try tailscale ip -4 first
    let output = exec.execute_shell("tailscale ip -4 2>/dev/null")?;

    if output.status.success() {
        let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !ip.is_empty() {
            return Ok(Some(ip));
        }
    }

    // Fallback: try tailscale status --json and extract IP from there
    // This is more reliable as it doesn't require the 'ip' subcommand
    let status_output = exec.execute_shell("tailscale status --json 2>/dev/null")?;
    if status_output.status.success() {
        if let Ok(status_json) = serde_json::from_slice::<serde_json::Value>(&status_output.stdout)
        {
            if let Some(self_data) = status_json.get("Self") {
                if let Some(ips) = self_data.get("TailscaleIPs").and_then(|v| v.as_array()) {
                    // Find IPv4 address (starts with 100.)
                    if let Some(ip) = ips.iter().find_map(|v| v.as_str()) {
                        if ip.starts_with("100.") {
                            return Ok(Some(ip.to_string()));
                        }
                    }
                }
            }
        }
    }

    Ok(None)
}

/// Get Tailscale IP with fallback to connection IP from config
/// This is the main function to use when you need a Tailscale IP and want automatic fallback
/// to the connection IP if commands fail (e.g., when connecting via Tailscale but commands have issues)
pub fn get_tailscale_ip_with_fallback<E: CommandExecutor>(
    exec: &E,
    hostname: &str,
    config: &EnvConfig,
) -> Result<String> {
    // First try to get IP via command
    let tailscale_ip = get_tailscale_ip_remote(exec)
        .context("Failed to get Tailscale IP. Ensure Tailscale is running.")?;

    if let Some(ip) = tailscale_ip {
        return Ok(ip);
    }

    // Fallback: check if we're connecting via Tailscale IP (100.x.x.x range)
    // If we successfully connected via SSH to a Tailscale address, Tailscale is working
    let host_config = get_host_config(config, hostname)?;

    // Check IP first (most reliable indicator)
    if let Some(ip) = &host_config.ip {
        if ip.starts_with("100.") {
            println!(
                "✓ Detected Tailscale connection (connecting via Tailscale IP: {})",
                ip
            );
            return Ok(ip.clone());
        }
    }

    // Also check hostname for Tailscale indicators
    if let Some(hostname_val) = &host_config.hostname {
        if hostname_val.ends_with(".ts.net") || hostname_val.contains("100.") {
            println!(
                "✓ Detected Tailscale connection (connecting via Tailscale hostname: {})",
                hostname_val
            );
            // Try to get actual IP, or use placeholder
            if let Some(ip) = &host_config.ip {
                if ip.starts_with("100.") {
                    return Ok(ip.clone());
                }
            }
            // Use placeholder - connection works via Tailscale
            return Ok("100.0.0.0".to_string());
        }
    }

    // Last resort: try tailscale status command to see if it's running
    let status_output = exec.execute_shell("tailscale status --json 2>/dev/null")?;
    if status_output.status.success() {
        // Try to extract IP from status JSON
        if let Ok(status_json) = serde_json::from_slice::<serde_json::Value>(&status_output.stdout)
        {
            if let Some(self_data) = status_json.get("Self") {
                if let Some(ips) = self_data.get("TailscaleIPs").and_then(|v| v.as_array()) {
                    if let Some(ip) = ips.iter().find_map(|v| v.as_str()) {
                        if ip.starts_with("100.") {
                            println!("✓ Tailscale is running with IP: {}", ip);
                            return Ok(ip.to_string());
                        }
                    }
                }
            }
        }
        println!("✓ Tailscale is running (detected via status command)");
        return Ok("100.0.0.0".to_string()); // Placeholder - status works but couldn't extract IP
    }

    anyhow::bail!(
        "Tailscale is not running or not accessible. Please ensure Tailscale is running with 'sudo tailscale up'"
    )
}

/// Get Tailscale hostname on a remote host via executor
pub fn get_tailscale_hostname_remote<E: CommandExecutor>(exec: &E) -> Result<Option<String>> {
    let output = exec.execute_shell("tailscale status --json 2>/dev/null")?;

    if output.status.success() {
        if let Ok(status_json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
            if let Some(dns_name) = status_json.get("Self").and_then(|s| s.get("DNSName")) {
                if let Some(hostname) = dns_name.as_str() {
                    // Strip trailing dot (DNS absolute notation) which causes SSH resolution issues
                    return Ok(Some(hostname.trim_end_matches('.').to_string()));
                }
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
                    // Strip trailing dot (DNS absolute notation) which causes SSH resolution issues
                    return Ok(Some(hostname.trim_end_matches('.').to_string()));
                }
            }
        }
        }
    }

    Ok(None)
}

/// Get Tailscale hostname for a specific peer by name
/// Looks up the peer in the local Tailscale status and returns its DNSName
pub fn get_peer_tailscale_hostname(peer_name: &str) -> Result<Option<String>> {
    let output = Command::new("tailscale")
        .args(&["status", "--json"])
        .output()
        .context("Failed to execute tailscale status")?;

    if !output.status.success() {
        return Ok(None);
    }

    let status_json: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("Failed to parse tailscale status JSON")?;

    // Check Self first (in case we're looking up our own hostname)
    if let Some(self_data) = status_json.get("Self") {
        if let Some(dns_name) = self_data.get("DNSName").and_then(|v| v.as_str()) {
            // Strip trailing dot (DNS absolute notation) which causes SSH resolution issues
            let hostname = dns_name.trim_end_matches('.');
            // Check if it matches (either full DNSName or short name)
            if hostname == peer_name || hostname.starts_with(&format!("{}.", peer_name)) {
                return Ok(Some(hostname.to_string()));
            }
        }
    }

    // Search through peers
    if let Some(peer_map) = status_json.get("Peer") {
        if let Some(peers) = peer_map.as_object() {
            for (_, peer_data) in peers {
                if let Some(dns_name) = peer_data.get("DNSName").and_then(|v| v.as_str()) {
                    // Strip trailing dot (DNS absolute notation) which causes SSH resolution issues
                    let hostname = dns_name.trim_end_matches('.');
                    // Match full DNSName or short name (before first dot)
                    if hostname == peer_name || hostname.starts_with(&format!("{}.", peer_name)) {
                        return Ok(Some(hostname.to_string()));
                    }
                }
            }
        }
    }

    Ok(None)
}
