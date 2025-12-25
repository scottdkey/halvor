//! PIA VPN Container Entrypoint
//! Rust-based entrypoint that replaces the bash entrypoint.sh script
//! Manages OpenVPN and Privoxy processes, downloads configs, and tails logs

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

mod config;
mod download;
mod logs;
mod process;
mod test;

use config::*;
use download::*;
use logs::*;
use process::*;
use test::*;

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
        return run_vpn_tests();
    }

    // Handle check-proxy command
    if cli.check_proxy {
        return check_proxy_status();
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
    configure_privoxy(&proxy_port)?;

    // Download configs if needed
    if env::var("UPDATE_CONFIGS").unwrap_or_default() == "true" {
        download_pia_configs()?;
    }

    // Fix existing config files
    fix_config_files()?;

    // Create auth.txt from environment variables
    create_auth_file()?;

    // Find OpenVPN config file
    let ovpn_config = find_ovpn_config()?;
    println!("Using OpenVPN config: {}", ovpn_config.display());

    // Check IPv6 availability
    check_ipv6()?;

    // Setup signal handlers
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    ctrlc::set_handler(move || {
        println!("\nShutting down...");
        running_clone.store(false, Ordering::SeqCst);
    })?;

    // Start Privoxy
    println!("Starting Privoxy...");
    let privoxy_pid = start_privoxy()?;
    println!("✓ Privoxy started (PID: {})", privoxy_pid);

    // Start OpenVPN
    println!("Starting OpenVPN...");
    start_openvpn(&ovpn_config)?;

    // Wait for OpenVPN to start
    println!("Waiting for OpenVPN connection...");
    thread::sleep(Duration::from_secs(8));

    // Verify OpenVPN is running
    let openvpn_pid = find_openvpn_pid(&ovpn_config)?;
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
    if let Ok(Some(vpn_ip)) = get_tun_interface_info() {
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
    check_openvpn_initialization()?;

    // Update DNS if needed
    update_dns()?;

    // Run connectivity test
    test_connectivity()?;

    println!();
    println!("VPN Status:");
    show_vpn_status()?;
    println!();
    println!("Proxy: http://0.0.0.0:{}", proxy_port);
    println!();

    // Run startup tests
    println!("=== Running Startup Tests ===");
    println!();
    run_startup_tests(&proxy_port)?;
    println!();

    println!("=== Service Logs ===");
    println!();

    // Start tailing logs
    let running_logs = running.clone();
    let openvpn_pid_logs = openvpn_pid;
    let privoxy_pid_logs = privoxy_pid;
    let ovpn_config_logs = ovpn_config.clone();

    thread::spawn(move || {
        tail_logs(
            running_logs,
            openvpn_pid_logs,
            privoxy_pid_logs,
            &ovpn_config_logs,
        );
    });

    // Monitor processes
    monitor_processes(running, openvpn_pid, privoxy_pid, &ovpn_config)?;

    Ok(())
}

fn configure_privoxy(port: &str) -> Result<()> {
    println!("Configuring Privoxy to listen on port {}...", port);

    // Read existing config
    let config = if Path::new(PRIVOXY_CONFIG).exists() {
        fs::read_to_string(PRIVOXY_CONFIG)?
    } else {
        String::new()
    };

    // Remove existing listen-address lines
    let lines: Vec<&str> = config.lines().collect();
    let filtered: Vec<&str> = lines
        .iter()
        .filter(|line| !line.trim_start().starts_with("listen-address"))
        .copied()
        .collect();

    // Rebuild config with new listen-address
    let mut new_config = filtered.join("\n");
    if !new_config.ends_with('\n') {
        new_config.push('\n');
    }
    new_config.push_str(&format!("listen-address 0.0.0.0:{}\n", port));

    fs::write(PRIVOXY_CONFIG, new_config)?;
    Ok(())
}

fn start_privoxy() -> Result<u32> {
    let mut cmd = Command::new("privoxy");
    cmd.args(&["--no-daemon", PRIVOXY_CONFIG]);
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());

    let child = cmd.spawn()?;
    Ok(child.id())
}

fn start_openvpn(config_path: &Path) -> Result<()> {
    let mut cmd = Command::new("openvpn");
    cmd.args(&[
        "--config",
        config_path.to_str().unwrap(),
        "--auth-user-pass",
        "/config/auth.txt",
        "--daemon",
        "--log",
        OPENVPN_LOG,
        "--pull-filter",
        "ignore",
        "ifconfig-ipv6",
        "--pull-filter",
        "ignore",
        "route-ipv6",
        "--mssfix",
        "1450",
        "--sndbuf",
        "393216",
        "--rcvbuf",
        "393216",
        "--verb",
        "3",
    ]);

    let output = cmd.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to start OpenVPN: {}", stderr);
    }
    Ok(())
}

fn find_openvpn_pid(config_path: &Path) -> Result<Option<u32>> {
    let output = Command::new("pgrep")
        .args(&["-f", &format!("openvpn.*{}", config_path.display())])
        .output()?;

    if output.status.success() {
        let pid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if let Ok(pid) = pid_str.parse::<u32>() {
            return Ok(Some(pid));
        }
    }
    Ok(None)
}

fn get_tun_interface_info() -> Result<Option<String>> {
    let output = Command::new("ip")
        .args(&["addr", "show", "tun0"])
        .output()?;

    if output.status.success() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            if line.contains("inet ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let ip = parts[1].split('/').next().unwrap_or("");
                    return Ok(Some(ip.to_string()));
                }
            }
        }
    }
    Ok(None)
}

fn check_openvpn_initialization() -> Result<()> {
    if !Path::new(OPENVPN_LOG).exists() {
        return Ok(());
    }

    let log_content = fs::read_to_string(OPENVPN_LOG)?;
    if log_content.contains("Initialization Sequence Completed") {
        println!("✓ OpenVPN initialization completed");
        thread::sleep(Duration::from_secs(2));
    } else {
        eprintln!("⚠ OpenVPN may not have completed initialization");
        eprintln!("Recent logs:");
        let lines: Vec<&str> = log_content.lines().rev().take(20).collect();
        for line in lines.iter().rev() {
            eprintln!("{}", line);
        }
    }
    Ok(())
}

fn update_dns() -> Result<()> {
    if !Path::new(OPENVPN_LOG).exists() {
        return Ok(());
    }

    let log_content = fs::read_to_string(OPENVPN_LOG)?;
    let dns_line = log_content
        .lines()
        .rev()
        .find(|line| line.contains("dhcp-option DNS"));

    if let Some(line) = dns_line {
        if let Some(dns_ip) = extract_dns_ip(line) {
            println!("Updating DNS to use VPN DNS server: {}", dns_ip);
            update_resolv_conf(&dns_ip)?;
            println!("✓ DNS updated");

            // Re-apply DNS after a delay
            let dns_ip_clone = dns_ip.clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_secs(5));
                if let Err(e) = update_resolv_conf(&dns_ip_clone) {
                    eprintln!("Failed to re-apply DNS: {}", e);
                } else {
                    println!("✓ DNS re-applied");
                }
            });
        }
    }
    Ok(())
}

fn extract_dns_ip(line: &str) -> Option<String> {
    // Extract IP from "dhcp-option DNS 1.2.3.4"
    let parts: Vec<&str> = line.split_whitespace().collect();
    for (i, part) in parts.iter().enumerate() {
        if part == &"DNS" && i + 1 < parts.len() {
            return Some(parts[i + 1].to_string());
        }
    }
    None
}

fn update_resolv_conf(dns_ip: &str) -> Result<()> {
    let backup_path = "/etc/resolv.conf.backup";
    if Path::new("/etc/resolv.conf").exists() {
        fs::copy("/etc/resolv.conf", backup_path)?;
    }

    let mut resolv = fs::File::create("/etc/resolv.conf")?;
    writeln!(resolv, "# VPN DNS from OpenVPN")?;
    writeln!(resolv, "nameserver {}", dns_ip)?;
    writeln!(resolv, "nameserver 8.8.8.8")?;
    writeln!(resolv, "nameserver 127.0.0.11")?;
    Ok(())
}

fn test_connectivity() -> Result<()> {
    println!("Running connectivity test...");
    let output = Command::new("curl")
        .args(&["-s", "--max-time", "5", "https://api.ipify.org"])
        .output()?;

    if output.status.success() {
        let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
        println!("✓ VPN connectivity verified - Public IP: {}", ip);
    } else {
        eprintln!("⚠ VPN connectivity test failed (may need more time)");
    }
    Ok(())
}

fn show_vpn_status() -> Result<()> {
    let output = Command::new("ip")
        .args(&["addr", "show", "tun0"])
        .output()?;

    if output.status.success() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            if line.contains("inet ") {
                println!("  VPN IP: {}", line.trim());
            }
        }
    } else {
        println!("  TUN interface: Not available");
    }
    Ok(())
}

fn run_startup_tests(proxy_port: &str) -> Result<()> {
    // Run comprehensive VPN tests
    std::env::set_var("PROXY_PORT", proxy_port);
    run_vpn_tests()?;
    Ok(())
}

fn monitor_processes(
    running: Arc<AtomicBool>,
    openvpn_pid: u32,
    privoxy_pid: u32,
    config_path: &Path,
) -> Result<()> {
    while running.load(Ordering::SeqCst) {
        // Check if OpenVPN is still running
        if let Ok(Some(current_pid)) = find_openvpn_pid(config_path) {
            if current_pid != openvpn_pid {
                // PID changed, but process exists
                continue;
            }
        } else {
            println!();
            eprintln!("⚠ OpenVPN process exited unexpectedly");
            if Path::new(OPENVPN_LOG).exists() {
                let log_content = fs::read_to_string(OPENVPN_LOG)?;
                let lines: Vec<&str> = log_content.lines().rev().take(30).collect();
                for line in lines.iter().rev() {
                    eprintln!("{}", line);
                }
            }
            cleanup(openvpn_pid, privoxy_pid, config_path)?;
            std::process::exit(1);
        }

        // Check if Privoxy is still running
        if let Err(_) = nix::sys::signal::kill(Pid::from_raw(privoxy_pid as i32), None) {
            println!();
            eprintln!("⚠ Privoxy exited");
            cleanup(openvpn_pid, privoxy_pid, config_path)?;
            std::process::exit(1);
        }

        thread::sleep(Duration::from_secs(5));
    }

    cleanup(openvpn_pid, privoxy_pid, config_path)?;
    Ok(())
}

fn cleanup(openvpn_pid: u32, privoxy_pid: u32, _config_path: &Path) -> Result<()> {
    println!("Stopping OpenVPN (PID: {})...", openvpn_pid);
    if let Err(e) = nix::sys::signal::kill(Pid::from_raw(openvpn_pid as i32), Signal::SIGTERM) {
        eprintln!("Failed to send SIGTERM to OpenVPN: {}", e);
    }

    thread::sleep(Duration::from_secs(2));

    // Force kill if still running
    if let Ok(_) = nix::sys::signal::kill(Pid::from_raw(openvpn_pid as i32), None) {
        let _ = nix::sys::signal::kill(Pid::from_raw(openvpn_pid as i32), Signal::SIGKILL);
    }

    println!("Stopping Privoxy (PID: {})...", privoxy_pid);
    if let Err(e) = nix::sys::signal::kill(Pid::from_raw(privoxy_pid as i32), Signal::SIGTERM) {
        eprintln!("Failed to send SIGTERM to Privoxy: {}", e);
    }

    Ok(())
}

fn check_ipv6() -> Result<()> {
    let output = Command::new("ip").args(&["-6", "addr", "show"]).output()?;

    if output.status.success() {
        println!("✓ IPv6 is available in container");
    } else {
        eprintln!(
            "⚠ IPv6 not available in container - IPv6 routes in OpenVPN config will be ignored"
        );
        eprintln!(
            "  To enable IPv6: Configure Docker daemon with IPv6 and set enable_ipv6: true in network"
        );
    }
    Ok(())
}
