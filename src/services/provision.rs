//! Provision Service
//!
//! Guided setup for new hosts in the halvor ecosystem.
//! Installs Docker, Tailscale, Portainer, and optional services.

use crate::config::EnvConfig;
use crate::services::compose_deployer::{deploy_compose_service, find_app};
use crate::services::docker;
use crate::services::tailscale;
use crate::utils::exec::{CommandExecutor, Executor};
use anyhow::{Context, Result};
use std::io::{self, BufRead, Write};

/// Provision with defaults (non-interactive)
pub fn provision_defaults(
    hostname: &str,
    portainer_host: bool,
    config: &EnvConfig,
) -> Result<()> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Provisioning {} (non-interactive mode)", hostname);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    let exec = Executor::new(hostname, config)?;
    let is_local = exec.is_local();

    // Check sudo access
    check_sudo_access(&exec, !is_local)?;

    // Install Docker
    println!("\n=== Installing Docker ===");
    docker::check_and_install(&exec)?;
    docker::configure_permissions(&exec)?;
    docker::configure_ipv6(&exec)?;

    // Install Tailscale
    println!("\n=== Installing Tailscale ===");
    tailscale::check_and_install_remote(&exec)?;

    // Install Portainer
    println!("\n=== Installing Portainer ===");
    let portainer_app = if portainer_host {
        find_app("portainer").unwrap()
    } else {
        find_app("portainer-agent").unwrap()
    };
    deploy_compose_service(hostname, portainer_app, config)?;

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✓ Provisioning complete for {}", hostname);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}

/// Guided interactive provisioning
pub fn provision_guided(hostname: &str, config: &EnvConfig) -> Result<()> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Halvor Guided Provisioning");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("This wizard will help you set up {} for the halvor ecosystem.", hostname);
    println!();

    let exec = Executor::new(hostname, config)?;
    let is_local = exec.is_local();

    if is_local {
        println!("Target: localhost (local execution)");
    } else {
        println!("Target: {} (remote execution via SSH)", hostname);
    }
    println!();

    // Check sudo access first
    check_sudo_access(&exec, !is_local)?;

    // Step 1: Docker
    println!();
    println!("┌─────────────────────────────────────────────────────────────────────────┐");
    println!("│ Step 1: Docker                                                          │");
    println!("└─────────────────────────────────────────────────────────────────────────┘");
    println!();
    println!("Docker is required to run containerized services.");

    if docker::is_docker_installed(&exec) {
        println!("✓ Docker is already installed");
        if prompt_yn("Configure Docker permissions and IPv6?", true)? {
            docker::configure_permissions(&exec)?;
            docker::configure_ipv6(&exec)?;
        }
    } else {
        if prompt_yn("Install Docker?", true)? {
            docker::check_and_install(&exec)?;
            docker::configure_permissions(&exec)?;
            docker::configure_ipv6(&exec)?;
        } else {
            println!("⚠ Skipping Docker - you'll need to install it manually to use services");
        }
    }

    // Step 2: Tailscale
    println!();
    println!("┌─────────────────────────────────────────────────────────────────────────┐");
    println!("│ Step 2: Tailscale (optional)                                            │");
    println!("└─────────────────────────────────────────────────────────────────────────┘");
    println!();
    println!("Tailscale provides secure remote access to your homelab.");

    if tailscale::is_tailscale_installed(&exec) {
        println!("✓ Tailscale is already installed");
    } else {
        if prompt_yn("Install Tailscale?", true)? {
            tailscale::check_and_install_remote(&exec)?;
        } else {
            println!("⚠ Skipping Tailscale");
        }
    }

    // Step 3: Portainer
    println!();
    println!("┌─────────────────────────────────────────────────────────────────────────┐");
    println!("│ Step 3: Portainer                                                       │");
    println!("└─────────────────────────────────────────────────────────────────────────┘");
    println!();
    println!("Portainer provides a web UI for managing Docker containers.");
    println!();
    println!("  Host:  Full Portainer with web UI (use on your main server)");
    println!("  Agent: Lightweight agent for remote management (use on other hosts)");
    println!();

    let portainer_choice = prompt_choice(
        "Install Portainer?",
        &["Host (with UI)", "Agent", "Skip"],
        0,
    )?;

    match portainer_choice {
        0 => {
            let app = find_app("portainer").unwrap();
            deploy_compose_service(hostname, app, config)?;
        }
        1 => {
            let app = find_app("portainer-agent").unwrap();
            deploy_compose_service(hostname, app, config)?;
        }
        _ => {
            println!("⚠ Skipping Portainer");
        }
    }

    // Step 4: Additional Services
    println!();
    println!("┌─────────────────────────────────────────────────────────────────────────┐");
    println!("│ Step 4: Additional Services                                             │");
    println!("└─────────────────────────────────────────────────────────────────────────┘");
    println!();

    if prompt_yn("Would you like to install additional services?", false)? {
        install_additional_services(hostname, config)?;
    }

    // Summary
    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✓ Provisioning complete for {}", hostname);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Next steps:");
    println!("  • Install more services:  halvor install <app>");
    println!("  • List available apps:    halvor install --list");
    println!("  • Check service status:   docker ps");

    Ok(())
}

/// Interactive service selection
fn install_additional_services(hostname: &str, config: &EnvConfig) -> Result<()> {
    // Group services by category
    println!();
    println!("Available services:");
    println!();

    // VPN and Reverse Proxy
    println!("  Infrastructure:");
    println!("    1. vpn                 - PIA VPN with HTTP proxy");
    println!("    2. nginx-proxy-manager - Reverse proxy with SSL");
    println!();

    // Media Stack
    println!("  Media Stack (requires VPN for privacy):");
    println!("    3. sabnzbd             - Usenet download client");
    println!("    4. qbittorrent         - Torrent download client");
    println!("    5. radarr              - Movie management");
    println!("    6. radarr-4k           - Movie management (4K)");
    println!("    7. sonarr              - TV show management");
    println!("    8. prowlarr            - Indexer manager");
    println!("    9. bazarr              - Subtitle management");
    println!();

    println!("Enter numbers separated by spaces (e.g., '1 3 5 7'), or 'all' for everything:");
    print!("> ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().lock().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    if input.is_empty() || input == "none" || input == "skip" {
        println!("No additional services selected.");
        return Ok(());
    }

    let services_to_install: Vec<&str> = if input == "all" {
        vec![
            "vpn",
            "nginx-proxy-manager",
            "sabnzbd",
            "qbittorrent",
            "radarr",
            "radarr-4k",
            "sonarr",
            "prowlarr",
            "bazarr",
        ]
    } else {
        let numbers: Vec<usize> = input
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();

        let service_map = [
            "vpn",
            "nginx-proxy-manager",
            "sabnzbd",
            "qbittorrent",
            "radarr",
            "radarr-4k",
            "sonarr",
            "prowlarr",
            "bazarr",
        ];

        numbers
            .iter()
            .filter_map(|&n| {
                if n >= 1 && n <= service_map.len() {
                    Some(service_map[n - 1])
                } else {
                    None
                }
            })
            .collect()
    };

    if services_to_install.is_empty() {
        println!("No valid services selected.");
        return Ok(());
    }

    println!();
    println!("Installing: {}", services_to_install.join(", "));
    println!();

    // Check if VPN is needed but not selected
    let needs_vpn: Vec<&&str> = services_to_install
        .iter()
        .filter(|s| {
            find_app(s)
                .map(|app| app.requires_vpn)
                .unwrap_or(false)
        })
        .collect();

    let has_vpn = services_to_install.contains(&"vpn");

    if !needs_vpn.is_empty() && !has_vpn {
        println!("⚠ Note: {} require VPN for privacy.", needs_vpn.iter().map(|s| **s).collect::<Vec<_>>().join(", "));
        if prompt_yn("Install VPN first?", true)? {
            if let Some(app) = find_app("vpn") {
                deploy_compose_service(hostname, app, config)?;
            }
        }
    }

    // Install selected services
    for service_name in services_to_install {
        if let Some(app) = find_app(service_name) {
            println!();
            deploy_compose_service(hostname, app, config)?;
        }
    }

    Ok(())
}

/// Check sudo access (works for both local and remote)
pub fn check_sudo_access<E: CommandExecutor>(exec: &E, is_remote: bool) -> Result<()> {
    println!("=== Checking sudo access ===");

    if !exec.is_linux()? {
        println!("✓ macOS detected (Docker Desktop handles permissions)");
        return Ok(());
    }

    if is_remote {
        // Remote execution: check for passwordless sudo
        let output = exec.execute_simple("sudo", &["-n", "true"])?;
        if !output.status.success() {
            println!("Error: Passwordless sudo is required for remote provisioning.");
            println!();
            println!("To configure passwordless sudo, run on the target host:");
            println!("  sudo visudo");
            println!();
            println!("Then add this line (replace USERNAME with your username):");
            println!("  USERNAME ALL=(ALL) NOPASSWD: ALL");
            println!();
            anyhow::bail!("Passwordless sudo not configured");
        }
        println!("✓ Passwordless sudo configured");
    } else {
        // Local execution: use interactive mode to prompt for password if needed
        println!("Testing sudo access (you may be prompted for your password)...");
        exec.execute_interactive("sudo", &["sh", "-c", "true"])?;
        println!("✓ Sudo access verified");
    }

    Ok(())
}

/// Prompt for yes/no with default
fn prompt_yn(question: &str, default: bool) -> Result<bool> {
    let default_hint = if default { "[Y/n]" } else { "[y/N]" };
    print!("{} {} ", question, default_hint);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().lock().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    if input.is_empty() {
        return Ok(default);
    }

    Ok(input.starts_with('y'))
}

/// Prompt for multiple choice
fn prompt_choice(question: &str, options: &[&str], default: usize) -> Result<usize> {
    println!("{}", question);
    for (i, option) in options.iter().enumerate() {
        let marker = if i == default { ">" } else { " " };
        println!("  {} {}. {}", marker, i + 1, option);
    }
    print!("Enter number [{}]: ", default + 1);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().lock().read_line(&mut input)?;
    let input = input.trim();

    if input.is_empty() {
        return Ok(default);
    }

    input
        .parse::<usize>()
        .ok()
        .and_then(|n| if n >= 1 && n <= options.len() { Some(n - 1) } else { None })
        .ok_or_else(|| anyhow::anyhow!("Invalid choice"))
}
