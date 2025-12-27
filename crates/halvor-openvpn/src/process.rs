//! Process management utilities

use anyhow::{Context, Result};
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

const OPENVPN_LOG: &str = "/var/log/openvpn/openvpn.log";
const PRIVOXY_CONFIG: &str = "/etc/privoxy/config";

/// Check if a process is running by PID
pub fn is_process_running(pid: u32) -> bool {
    nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid as i32), None).is_ok()
}

/// Find process ID by command pattern
pub fn find_process_by_pattern(pattern: &str) -> Result<Option<u32>> {
    let output = Command::new("pgrep").args(&["-f", pattern]).output()?;

    if output.status.success() {
        let pid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if let Ok(pid) = pid_str.parse::<u32>() {
            return Ok(Some(pid));
        }
    }
    Ok(None)
}

/// Start Privoxy in background
pub fn start_privoxy() -> Result<u32> {
    let mut cmd = Command::new("privoxy");
    cmd.args(&["--no-daemon", PRIVOXY_CONFIG]);
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());

    let child = cmd.spawn()?;
    Ok(child.id())
}

/// Start OpenVPN with specified config
pub fn start_openvpn(config_path: &Path) -> Result<()> {
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

/// Find OpenVPN process ID by config path
pub fn find_openvpn_pid(config_path: &Path) -> Result<Option<u32>> {
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

/// Get TUN interface information
pub fn get_tun_interface_info() -> Result<Option<String>> {
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

/// Check if OpenVPN initialization completed
pub fn check_openvpn_initialization() -> Result<()> {
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

/// Update DNS to use VPN DNS server
pub fn update_dns() -> Result<()> {
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

/// Extract DNS IP from log line
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

/// Update resolv.conf with VPN DNS
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

/// Monitor OpenVPN and Privoxy processes
pub fn monitor_processes(
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

/// Cleanup processes on shutdown
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

