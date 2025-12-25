//! VPN testing and diagnostics functionality

use anyhow::{Context, Result};
use if_addrs::get_if_addrs;
use std::net::TcpStream;
use std::process::Command;
use std::time::Duration;

const PROXY_PORT_ENV: &str = "PROXY_PORT";

/// Run comprehensive VPN connection tests
pub fn run_vpn_tests() -> Result<()> {
    let proxy_port = std::env::var(PROXY_PORT_ENV).unwrap_or_else(|_| "8888".to_string());

    println!("=== VPN Connection Test ===");
    println!();

    // Test 1: Check if OpenVPN is running
    println!("1. Checking OpenVPN status...");
    let openvpn_pid = find_openvpn_pid()?;
    if let Some(pid) = openvpn_pid {
        println!("   ✓ OpenVPN is running");
        println!("   PID: {}", pid);
    } else {
        eprintln!("   ✗ OpenVPN is not running");
        return Ok(());
    }
    println!();

    // Test 2: Check TUN interface
    println!("2. Checking TUN interface...");
    let vpn_ip = get_tun_interface_ip()?;
    if let Some(ip) = &vpn_ip {
        println!("   ✓ tun0 interface exists");
        println!("   VPN IP: {}", ip);
    } else {
        eprintln!("   ✗ tun0 interface not found");
        return Ok(());
    }
    println!();

    // Test 3: Check routing
    println!("3. Checking routing table...");
    let default_route = get_default_route()?;
    println!("   Default route: {}", default_route);
    let vpn_routes = count_vpn_routes()?;
    println!("   Routes via tun0: {}", vpn_routes);
    if vpn_routes > 0 {
        println!("   ✓ Traffic is routed through VPN");
    } else {
        eprintln!("   ⚠ Warning: No routes via tun0 found");
    }
    println!();

    // Test 4: Get public IP (direct, no proxy)
    println!("4. Testing direct connection (should show VPN IP)...");
    let direct_ip = get_public_ip_direct()?;
    if let Some(ip) = &direct_ip {
        println!("   Direct IP: {}", ip);
        println!("   ✓ Direct connection working");
        if vpn_ip.is_some() {
            println!("   Note: This IP should be different from your host's public IP");
        }
    } else {
        eprintln!("   ✗ Direct connection failed");
    }
    println!();

    // Test 5: Get public IP via Privoxy proxy
    println!("5. Testing connection via Privoxy proxy...");
    if is_privoxy_running()? {
        let proxy_ip = get_public_ip_via_proxy(&proxy_port)?;
        if let Some(ip) = &proxy_ip {
            println!("   Proxy IP: {}", ip);
            println!("   ✓ Proxy connection working");

            // Compare IPs
            if direct_ip.as_ref() == Some(ip) && direct_ip.is_some() {
                println!("   ✓ Both direct and proxy show same IP (VPN is working)");
            } else if direct_ip.is_some() && proxy_ip.is_some() {
                eprintln!("   ⚠ Warning: Direct and proxy IPs differ");
            }
        } else {
            eprintln!("   ✗ Proxy connection failed");
        }
    } else {
        eprintln!("   ✗ Privoxy is not running");
    }
    println!();

    // Test 6: Test DNS resolution
    println!("6. Testing DNS resolution...");
    if test_dns_resolution()? {
        println!("   ✓ DNS resolution working");
    } else {
        eprintln!("   ⚠ DNS resolution test failed (may be normal)");
    }
    println!();

    // Test 7: Test HTTP connectivity
    println!("7. Testing HTTP connectivity...");
    if test_http_connectivity()? {
        println!("   ✓ HTTP connectivity working");
    } else {
        eprintln!("   ✗ HTTP connectivity failed");
    }
    println!();

    // Test 8: Test HTTPS via proxy
    println!("8. Testing HTTPS via proxy...");
    if test_https_via_proxy(&proxy_port)? {
        println!("   ✓ HTTPS via proxy working");
    } else {
        eprintln!("   ✗ HTTPS via proxy failed");
    }
    println!();

    // Summary
    println!("=== Test Summary ===");
    if direct_ip.is_some() && vpn_ip.is_some() {
        println!("✓ VPN is connected and working");
        if let Some(vpn_ip) = vpn_ip {
            println!("  Container VPN IP: {}", vpn_ip);
        }
        if let Some(direct_ip) = direct_ip {
            println!("  Public IP (via VPN): {}", direct_ip);
        }
        println!();
        println!("To test from host:");
        println!(
            "  curl --proxy http://<host-ip>:{} https://api.ipify.org",
            proxy_port
        );
    } else {
        eprintln!("✗ VPN connection test failed");
    }

    Ok(())
}

/// Check proxy connectivity and network status
pub fn check_proxy_status() -> Result<()> {
    let proxy_port = std::env::var(PROXY_PORT_ENV).unwrap_or_else(|_| "8888".to_string());

    println!("=== Network Status ===");
    println!();

    println!("Container Network Interfaces:");
    let interfaces = get_network_interfaces()?;
    for line in interfaces.lines() {
        println!("  {}", line);
    }
    println!();

    println!("Routing Table:");
    let routes = get_routing_table()?;
    for line in routes.lines() {
        println!("  {}", line);
    }
    println!();

    println!("OpenVPN Status:");
    if let Some(pid) = find_openvpn_pid()? {
        println!("  ✓ OpenVPN is running");
        println!("  PID: {}", pid);
    } else {
        eprintln!("  ✗ OpenVPN is not running");
    }
    println!();

    println!("TUN Interface (VPN):");
    if let Some(vpn_ip) = get_tun_interface_ip()? {
        println!("  ✓ tun0 exists");
        println!("  VPN IP: {}", vpn_ip);
    } else {
        eprintln!("  ✗ tun0 not found");
    }
    println!();

    println!("Privoxy Status:");
    if let Some(pid) = find_privoxy_pid()? {
        println!("  ✓ Privoxy is running");
        println!("  PID: {}", pid);
    } else {
        eprintln!("  ✗ Privoxy is not running");
    }
    println!();

    println!("Port {} Listening:", proxy_port);
    let listening = get_listening_ports(&proxy_port)?;
    for line in listening.lines() {
        println!("  {}", line);
    }
    println!();

    println!("Proxy Test:");
    println!("  Testing local connection...");
    if let Some(ip) = get_public_ip_via_proxy(&proxy_port)? {
        println!("  ✓ Proxy is working");
        println!("  VPN IP (via proxy): {}", ip);
    } else {
        eprintln!("  ✗ Proxy connection failed");
    }
    println!();

    println!("Public IP (direct, no proxy):");
    if let Some(ip) = get_public_ip_direct()? {
        println!("  {}", ip);
    } else {
        println!("  Unable to determine");
    }
    println!();

    println!("=== Access Information ===");
    println!("  From host: http://<host-ip>:{}", proxy_port);
    println!("  From containers: http://pia-vpn:{}", proxy_port);
    println!(
        "  Example: curl --proxy http://<host-ip>:{} https://api.ipify.org",
        proxy_port
    );
    println!();

    Ok(())
}

// Helper functions

fn find_openvpn_pid() -> Result<Option<u32>> {
    // Use nix to find processes by reading /proc
    use std::fs;
    use std::path::Path;

    let proc_dir = Path::new("/proc");
    if !proc_dir.exists() {
        return Ok(None);
    }

    for entry in fs::read_dir(proc_dir)? {
        let entry = entry?;
        let pid_str = entry.file_name().to_string_lossy().to_string();
        if let Ok(pid) = pid_str.parse::<u32>() {
            // Check cmdline
            let cmdline_path = entry.path().join("cmdline");
            if let Ok(cmdline) = fs::read_to_string(&cmdline_path) {
                if cmdline.contains("openvpn") {
                    return Ok(Some(pid));
                }
            }
        }
    }
    Ok(None)
}

fn find_privoxy_pid() -> Result<Option<u32>> {
    // Use nix to find processes by reading /proc
    use std::fs;
    use std::path::Path;

    let proc_dir = Path::new("/proc");
    if !proc_dir.exists() {
        return Ok(None);
    }

    for entry in fs::read_dir(proc_dir)? {
        let entry = entry?;
        let pid_str = entry.file_name().to_string_lossy().to_string();
        if let Ok(pid) = pid_str.parse::<u32>() {
            // Check cmdline
            let cmdline_path = entry.path().join("cmdline");
            if let Ok(cmdline) = fs::read_to_string(&cmdline_path) {
                if cmdline.contains("privoxy") {
                    return Ok(Some(pid));
                }
            }
        }
    }
    Ok(None)
}

fn is_privoxy_running() -> Result<bool> {
    Ok(find_privoxy_pid()?.is_some())
}

fn get_tun_interface_ip() -> Result<Option<String>> {
    // Use if_addrs crate to get network interfaces
    let addrs = get_if_addrs().context("Failed to get network interfaces")?;

    for iface in addrs {
        if iface.name == "tun0" {
            if let if_addrs::IfAddr::V4(ipv4) = iface.addr {
                return Ok(Some(ipv4.ip.to_string()));
            }
        }
    }
    Ok(None)
}

fn get_default_route() -> Result<String> {
    let output = Command::new("ip")
        .args(&["route", "show", "default"])
        .output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Ok("No default route".to_string())
    }
}

fn count_vpn_routes() -> Result<usize> {
    let output = Command::new("ip").args(&["route", "show"]).output()?;

    if output.status.success() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        let count = output_str
            .lines()
            .filter(|line| line.contains("via") && line.contains("tun0"))
            .count();
        Ok(count)
    } else {
        Ok(0)
    }
}

fn get_public_ip_direct() -> Result<Option<String>> {
    // Use reqwest instead of curl
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let response = client.get("https://api.ipify.org").send()?;
    let ip = response.text()?.trim().to_string();

    if !ip.is_empty() && ip != "Failed" {
        Ok(Some(ip))
    } else {
        Ok(None)
    }
}

fn get_public_ip_via_proxy(proxy_port: &str) -> Result<Option<String>> {
    // Use reqwest with proxy instead of curl
    let proxy_url = format!("http://127.0.0.1:{}", proxy_port);
    let proxy = reqwest::Proxy::http(&proxy_url)?;

    let client = reqwest::blocking::Client::builder()
        .proxy(proxy)
        .timeout(Duration::from_secs(10))
        .build()?;

    let response = client.get("https://api.ipify.org").send()?;
    let ip = response.text()?.trim().to_string();

    if !ip.is_empty() && ip != "Failed" {
        Ok(Some(ip))
    } else {
        Ok(None)
    }
}

fn test_dns_resolution() -> Result<bool> {
    // Use std::net::ToSocketAddrs for DNS resolution
    use std::net::ToSocketAddrs;
    let addr = ("google.com", 80).to_socket_addrs()?;
    Ok(addr.count() > 0)
}

fn test_http_connectivity() -> Result<bool> {
    // Use reqwest instead of curl
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let response = client.get("http://www.google.com").send()?;
    Ok(response.status().is_success())
}

fn test_https_via_proxy(proxy_port: &str) -> Result<bool> {
    // Use reqwest with proxy instead of curl
    let proxy_url = format!("http://127.0.0.1:{}", proxy_port);
    let proxy = reqwest::Proxy::https(&proxy_url)?;

    let client = reqwest::blocking::Client::builder()
        .proxy(proxy)
        .timeout(Duration::from_secs(10))
        .build()?;

    let response = client.get("https://www.google.com").send()?;
    Ok(response.status().is_success())
}

fn get_network_interfaces() -> Result<String> {
    // Use if_addrs crate to get network interfaces
    let addrs = get_if_addrs().context("Failed to get network interfaces")?;
    let mut result = Vec::new();

    for iface in addrs {
        match iface.addr {
            if_addrs::IfAddr::V4(ipv4) => {
                result.push(format!("{}: inet {}", iface.name, ipv4.ip));
            }
            if_addrs::IfAddr::V6(ipv6) => {
                result.push(format!("{}: inet6 {}", iface.name, ipv6.ip));
            }
        }
    }

    Ok(result.join("\n"))
}

fn get_routing_table() -> Result<String> {
    // Reading routing table requires root or parsing /proc/net/route
    // For now, keep using ip command as it's the most reliable way
    use std::process::Command;
    let output = Command::new("ip").arg("route").output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Ok(String::new())
    }
}

fn get_listening_ports(port: &str) -> Result<String> {
    // Try to connect to the port to see if it's listening
    let port_num: u16 = port.parse().context("Invalid port number")?;
    let addr = format!("127.0.0.1:{}", port_num);

    match TcpStream::connect_timeout(&addr.parse()?, Duration::from_millis(100)) {
        Ok(_) => Ok(format!("Port {} is listening", port)),
        Err(_) => Ok(format!("Port {} is not accessible", port)),
    }
}
