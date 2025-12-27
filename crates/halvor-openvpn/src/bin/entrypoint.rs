//! PIA VPN Container Entrypoint
//! Rust-based entrypoint that replaces the bash entrypoint.sh script
//! Manages OpenVPN and Privoxy processes, downloads configs, and tails logs

use halvor_openvpn::{config, download, logs, process, test};
use anyhow::{Context, Result};
use clap::Parser;
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

const PIA_CONFIG_URL: &str = "https://www.privateinternetaccess.com/openvpn/openvpn.zip";
const CONFIG_DIR: &str = "/config";
const OPENVPN_LOG: &str = "/var/log/openvpn/openvpn.log";
const PRIVOXY_LOG: &str = "/var/log/privoxy/logfile";
const PRIVOXY_CONFIG: &str = "/etc/privoxy/config";

#[derive(Parser)]
#[command(name = "entrypoint")]
#[command(about = "PIA VPN Container Entrypoint")]
struct Cli {
    /// Run VPN tests and exit
    #[arg(long)]
    test: bool,
    /// Check proxy status and exit
    #[arg(long)]
    check_proxy: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Handle test command
    if cli.test {
        let proxy_port = env::var("PROXY_PORT").unwrap_or_else(|_| "8888".to_string());
        std::env::set_var("PROXY_PORT", &proxy_port);
        return test::run_vpn_tests();
    }

    // Handle check-proxy command
    if cli.check_proxy {
        return test::check_proxy_status();
    }

    // Otherwise, run normal entrypoint
    run_entrypoint()
}

fn run_entrypoint() -> Result<()> {
    // Print environment variables if DEBUG is set
    if env::var("DEBUG").unwrap_or_default() == "true" {
        println!("=== Environment Variables ===");
        println!(
            "REGION: {}",
            env::var("REGION").unwrap_or_else(|_| "<not set>".to_string())
        );
        println!(
            "PIA_USERNAME: {}",
            env::var("PIA_USERNAME").unwrap_or_else(|_| "<not set>".to_string())
        );
        println!(
            "PIA_PASSWORD: {}",
            if env::var("PIA_PASSWORD").is_ok() {
                "<set>"
            } else {
                "<not set>"
            }
        );
        println!(
            "UPDATE_CONFIGS: {}",
            env::var("UPDATE_CONFIGS").unwrap_or_else(|_| "<not set>".to_string())
        );
        println!(
            "PROXY_PORT: {}",
            env::var("PROXY_PORT").unwrap_or_else(|_| "8888".to_string())
        );
        println!(
            "TZ: {}",
            env::var("TZ").unwrap_or_else(|_| "<not set>".to_string())
        );
        println!("==============================");
        println!();
    }

    // Get proxy port
    let proxy_port = env::var("PROXY_PORT").unwrap_or_else(|_| "8888".to_string());

    // Configure Privoxy
    config::configure_privoxy(&proxy_port)?;

    // Download configs if needed
    if env::var("UPDATE_CONFIGS").unwrap_or_default() == "true" {
        download::download_pia_configs()?;
    }

    // Fix existing config files
    config::fix_config_files()?;

    // Create auth.txt from environment variables
    config::create_auth_file()?;

    // Find OpenVPN config file
    let ovpn_config = config::find_ovpn_config()?;
    println!("Using OpenVPN config: {}", ovpn_config.display());

    // Check IPv6 availability
    test::check_ipv6()?;

    // Setup signal handlers
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    ctrlc::set_handler(move || {
        println!("\nShutting down...");
        running_clone.store(false, Ordering::SeqCst);
    })?;

    // Start Privoxy
    println!("Starting Privoxy...");
    let privoxy_pid = process::start_privoxy()?;
    println!("✓ Privoxy started (PID: {})", privoxy_pid);

    // Start OpenVPN
    println!("Starting OpenVPN...");
    process::start_openvpn(&ovpn_config)?;

    // Wait for OpenVPN to start
    println!("Waiting for OpenVPN connection...");
    thread::sleep(Duration::from_secs(8));

    // Verify OpenVPN is running
    let openvpn_pid = process::find_openvpn_pid(&ovpn_config)?;
    if openvpn_pid.is_none() {
        eprintln!("⚠ OpenVPN process not found, checking logs...");
        if Path::new(OPENVPN_LOG).exists() {
            let log_content = fs::read_to_string(OPENVPN_LOG)?;
            let lines: Vec<&str> = log_content.lines().rev().take(20).collect();
            for line in lines.iter().rev() {
                eprintln!("{}", line);
            }
        }
        anyhow::bail!("OpenVPN failed to start");
    }
    let openvpn_pid = openvpn_pid.unwrap();
    println!("✓ OpenVPN started (PID: {})", openvpn_pid);

    // Check TUN interface (non-blocking, just for info)
    if let Ok(Some(vpn_ip)) = process::get_tun_interface_info() {
        println!("✓ TUN interface (tun0) found - VPN IP: {}", vpn_ip);
    } else {
        eprintln!("⚠ TUN interface (tun0) not found");
        eprintln!("OpenVPN may not have connected successfully");
        if Path::new(OPENVPN_LOG).exists() {
            let log_content = fs::read_to_string(OPENVPN_LOG)?;
            let lines: Vec<&str> = log_content.lines().rev().take(30).collect();
            for line in lines.iter().rev() {
                eprintln!("{}", line);
            }
        }
    }

    // Check for initialization completion
    process::check_openvpn_initialization()?;

    // Update DNS if needed
    process::update_dns()?;

    // Run connectivity test
    test::test_connectivity()?;

    println!();
    println!("VPN Status:");
    test::show_vpn_status()?;
    println!();
    println!("Proxy: http://0.0.0.0:{}", proxy_port);
    println!();

    // Run startup tests
    println!("=== Running Startup Tests ===");
    println!();
    test::run_startup_tests(&proxy_port)?;
    println!();

    println!("=== Service Logs ===");
    println!();

    // Start tailing logs
    let running_logs = running.clone();
    let openvpn_pid_logs = openvpn_pid;
    let privoxy_pid_logs = privoxy_pid;
    let ovpn_config_logs = ovpn_config.clone();

    thread::spawn(move || {
        logs::tail_logs(
            running_logs,
            openvpn_pid_logs,
            privoxy_pid_logs,
            &ovpn_config_logs,
        );
    });

    // Monitor processes
    process::monitor_processes(running, openvpn_pid, privoxy_pid, &ovpn_config)?;

    Ok(())
}

