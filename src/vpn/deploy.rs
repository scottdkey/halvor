use crate::exec::{CommandExecutor, Executor};
use anyhow::{Context, Result};
use std::env;

pub fn deploy_vpn(hostname: &str, config: &crate::config::EnvConfig) -> Result<()> {
    let homelab_dir = crate::config::find_homelab_dir()?;

    // Load PIA credentials from local .env
    dotenv::from_path(homelab_dir.join(".env")).context("Failed to load .env file")?;

    let pia_username = env::var("PIA_USERNAME").context("PIA_USERNAME not found in .env file")?;
    let pia_password = env::var("PIA_PASSWORD").context("PIA_PASSWORD not found in .env file")?;

    // Create executor - it automatically determines if execution should be local or remote
    let exec = Executor::new(hostname, config)?;
    let target_host = exec.target_host(hostname, config)?;
    let is_local = exec.is_local();

    if is_local {
        println!("Deploying VPN locally on {}...", hostname);
    } else {
        println!("Deploying VPN to {} ({})...", hostname, target_host);
    }
    println!();

    // Read compose file - use local build version for now (avoids registry auth issues)
    // User can switch to portainer version after making image public
    let compose_file = homelab_dir
        .join("compose")
        .join("openvpn-pia.docker-compose.yml");
    if !compose_file.exists() {
        anyhow::bail!("VPN compose file not found at {}", compose_file.display());
    }

    let compose_content = std::fs::read_to_string(&compose_file)
        .with_context(|| format!("Failed to read compose file: {}", compose_file.display()))?;

    // Don't substitute - let docker-compose read from .env file using --env-file

    // Determine username for VPN config path
    let default_user = crate::config::get_default_username();
    // Allow VPN_USER to override the username for config path (useful for Portainer)
    // If not set, uses the default user
    let vpn_user = env::var("VPN_USER").unwrap_or_else(|_| default_user.clone());

    // Check if files already exist - if so, skip deployment
    // Check for both .ovpn and .opvn (typo) variants
    // Use /home/$USER/config/vpn (USER can be set via VPN_USER env var)
    let vpn_config_dir = format!("/home/{}/config/vpn", vpn_user);
    let auth_exists = exec.file_exists(&format!("{}/auth.txt", vpn_config_dir))?;
    let config_exists = exec.file_exists(&format!("{}/ca-montreal.ovpn", vpn_config_dir))?
        || exec.file_exists(&format!("{}/ca-montreal.opvn", vpn_config_dir))?;
    let files_exist = auth_exists && config_exists;

    if files_exist {
        if is_local {
            println!("✓ VPN configuration files already exist");
        } else {
            println!("✓ VPN configuration files already exist on remote system");
        }
        println!("  Skipping file copy (files are already in place)");
    } else {
        println!("VPN configuration files not found, attempting to copy...");

        // Copy OpenVPN config files
        let openvpn_dir = homelab_dir.join("openvpn");
        let auth_file = openvpn_dir.join("auth.txt");
        let config_file = openvpn_dir.join("ca-montreal.ovpn");

        if !auth_file.exists() {
            anyhow::bail!("OpenVPN auth file not found at {}", auth_file.display());
        }
        if !config_file.exists() {
            anyhow::bail!("OpenVPN config file not found at {}", config_file.display());
        }

        // Read auth file and write directly
        let auth_content = std::fs::read(&auth_file)
            .with_context(|| format!("Failed to read auth file: {}", auth_file.display()))?;

        // Create directory and write file
        exec.mkdir_p(&vpn_config_dir)?;
        exec.write_file(&format!("{}/auth.txt", vpn_config_dir), &auth_content)?;
        exec.execute_shell_interactive(&format!("chmod 600 {}/auth.txt", vpn_config_dir))?;
        if is_local {
            println!("✓ Copied auth.txt");
        } else {
            println!("✓ Copied auth.txt to remote system");
        }

        // Copy config file
        let config_content = std::fs::read(&config_file)
            .with_context(|| format!("Failed to read config file: {}", config_file.display()))?;

        exec.write_file(
            &format!("{}/ca-montreal.ovpn", vpn_config_dir),
            &config_content,
        )?;
        exec.execute_shell_interactive(&format!("chmod 644 {}/ca-montreal.ovpn", vpn_config_dir))?;
        if is_local {
            println!("✓ Copied ca-montreal.ovpn");
        } else {
            println!("✓ Copied ca-montreal.ovpn to remote system");
        }
    }

    // Copy compose file (keep in home directory for user access)
    exec.mkdir_p("$HOME/vpn")?;
    exec.write_file("$HOME/vpn/docker-compose.yml", compose_content.as_bytes())?;
    if is_local {
        println!("✓ Copied VPN compose file");
    } else {
        println!("✓ Copied VPN compose file to remote system");
    }

    // Create .env file with PIA credentials
    let env_content = format!(
        "PIA_USERNAME={}\nPIA_PASSWORD={}\n",
        pia_username, pia_password
    );
    exec.write_file("$HOME/vpn/.env", env_content.as_bytes())?;
    if is_local {
        println!("✓ Created .env file");
    } else {
        println!("✓ Created .env file on remote system");
    }

    println!();
    println!(
        "✓ VPN configuration files copied to {} ({})",
        hostname, target_host
    );
    println!("  Files copied:");
    println!("    - ~/vpn/docker-compose.yml (Portainer compose file)");
    println!("    - ~/vpn/.env (PIA credentials)");
    println!(
        "    - /home/{}/config/vpn/auth.txt (OpenVPN authentication)",
        vpn_user
    );
    println!(
        "    - /home/{}/config/vpn/ca-montreal.ovpn (OpenVPN configuration)",
        vpn_user
    );
    println!();
    println!("  Note: Set USER environment variable in Portainer to match the username");
    println!("        Example: USER={}", vpn_user);
    println!();
    println!("  You can now deploy the VPN manually using Portainer or docker-compose.");

    Ok(())
}
