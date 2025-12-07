use crate::exec::{CommandExecutor, Executor, local};
use crate::vpn::utils;
use anyhow::Result;

pub fn verify_vpn(hostname: &str, config: &crate::config::EnvConfig) -> Result<()> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  VPN Verification for {}", hostname);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Create executor - it automatically determines if execution should be local or remote
    let exec = Executor::new(hostname, config)?;
    let target_host = exec.target_host(hostname, config)?;
    let is_local = exec.is_local();

    if is_local {
        println!("Detected local execution on {}", hostname);
    } else {
        println!("Detected remote execution - verifying via SSH");
    }
    println!();

    verify_vpn_with_executor(hostname, &target_host, &exec)
}

fn verify_vpn_with_executor(hostname: &str, target_host: &str, exec: &Executor) -> Result<()> {
    let mut all_passed = true;

    // Test 1: Check if container is running
    println!("[1/10] Checking VPN container status...");
    use crate::docker;
    if docker::is_container_running(exec, "openvpn-pia")? {
        println!("   ✓ VPN container is running");
    } else {
        println!("   ✗ VPN container is not running");
        println!();
        anyhow::bail!(
            "VPN container not found. Deploy VPN first with: hal vpn deploy {}",
            hostname
        );
    }

    // Test 2: Check OpenVPN process
    println!("[2/10] Checking OpenVPN process...");
    let openvpn_check = exec.execute_shell("docker exec openvpn-pia pgrep -f openvpn")?;
    if openvpn_check.status.success() {
        let pid = String::from_utf8_lossy(&openvpn_check.stdout)
            .trim()
            .to_string();
        println!("   ✓ OpenVPN is running (PID: {})", pid);
    } else {
        println!("   ✗ OpenVPN is not running");
        all_passed = false;
    }

    // Test 3: Check TUN interface
    println!("[3/10] Checking TUN interface...");
    let tun_check = exec.execute_shell("docker exec openvpn-pia ip addr show tun0 2>&1")?;
    if tun_check.status.success() {
        let tun_output = String::from_utf8_lossy(&tun_check.stdout);
        if let Some(ip_line) = tun_output.lines().find(|l| l.contains("inet ")) {
            if let Some(ip_part) = ip_line.split_whitespace().nth(1) {
                let ip = ip_part.split('/').next().unwrap_or(ip_part);
                println!("   ✓ TUN interface is up (IP: {})", ip);
            } else {
                println!("   ✓ TUN interface is up");
            }
        } else {
            println!("⚠ TUN interface exists but no IP found");
        }
    } else {
        println!("✗ TUN interface not found");
        all_passed = false;
    }

    // Test 4: Check routing
    println!("[4/10] Checking routing configuration...");
    let route_check = exec.execute_shell(
        "docker exec openvpn-pia ip route | grep -E '0\\.0\\.0\\.0/1|128\\.0\\.0\\.0/1'",
    )?;
    if route_check.status.success() {
        let route_output = String::from_utf8_lossy(&route_check.stdout);
        if route_output.contains("tun0") {
            println!("✓ Traffic is routed through VPN");
        } else {
            println!("⚠ Warning: Routes may not be configured correctly");
            all_passed = false;
        }
    } else {
        println!("⚠ Warning: Could not verify routing");
    }

    // Test 5: Check Privoxy
    println!("[5/10] Checking Privoxy process...");
    let privoxy_check = exec.execute_shell("docker exec openvpn-pia pgrep privoxy")?;
    if privoxy_check.status.success() {
        let pid = String::from_utf8_lossy(&privoxy_check.stdout)
            .trim()
            .to_string();
        println!("✓ Privoxy is running (PID: {})", pid);
    } else {
        println!("✗ Privoxy is not running");
        all_passed = false;
    }

    // Test 6: Check Privoxy port
    println!("[6/10] Checking Privoxy port 8888...");
    let port_check = exec.execute_shell("docker exec openvpn-pia ss -tlnp 2>/dev/null | grep 8888 || docker exec openvpn-pia netstat -tlnp 2>/dev/null | grep 8888")?;
    if port_check.status.success() {
        println!("✓ Privoxy is listening on port 8888");
    } else {
        println!("✗ Privoxy port 8888 not found");
        println!("Host: {}", hostname);
        all_passed = false;
    }

    // Test 7: Test DNS resolution
    println!("[7/10] Testing DNS resolution...");
    let dns_check =
        exec.execute_shell("docker exec openvpn-pia nslookup api.ipify.org 2>&1 | head -5")?;
    if dns_check.status.success() {
        let dns_output = String::from_utf8_lossy(&dns_check.stdout);
        if dns_output.contains("Name:") || dns_output.contains("Address:") {
            println!("✓ DNS resolution working");
        } else {
            println!("⚠ DNS resolution may have issues");
        }
    } else {
        println!("⚠ DNS resolution test failed");
    }

    // Test 8: Test direct connectivity (should show VPN IP)
    println!("[8/10] Testing direct connectivity (should show VPN IP)...");
    let direct_ip =
        exec.execute_shell("docker exec openvpn-pia curl -s --max-time 10 https://api.ipify.org")?;
    if direct_ip.status.success() {
        let ip_output = String::from_utf8_lossy(&direct_ip.stdout)
            .trim()
            .to_string();
        if !ip_output.is_empty() {
            println!("✓ Direct connection working (Public IP: {})", ip_output);
        } else {
            println!("✗ Direct connection returned empty response");
            all_passed = false;
        }
    } else {
        println!("   ✗ Direct connection failed");
        all_passed = false;
    }

    // Test 9: Test proxy connectivity
    println!("[9/10] Testing proxy connectivity...");
    let proxy_ip = exec.execute_shell("docker exec openvpn-pia curl -s --proxy http://127.0.0.1:8888 --max-time 10 https://api.ipify.org")?;
    if proxy_ip.status.success() {
        let proxy_output = String::from_utf8_lossy(&proxy_ip.stdout).trim().to_string();
        if !proxy_output.is_empty() {
            println!("✓ Proxy connection working (Public IP: {})", proxy_output);
        } else {
            println!("✗ Proxy connection returned empty response");
            all_passed = false;
        }
    } else {
        println!("✗ Proxy connection failed");
        all_passed = false;
    }

    // Test 10: Test from host
    println!("[10/10] Testing proxy from host...");
    let host_proxy = local::execute(
        "curl",
        &[
            "-s",
            "--proxy",
            &format!("http://{}:8888", target_host),
            "--max-time",
            "10",
            "https://api.ipify.org",
        ],
    )?;
    if host_proxy.status.success() {
        let host_output = String::from_utf8_lossy(&host_proxy.stdout)
            .trim()
            .to_string();
        if !host_output.is_empty() {
            println!(
                "   ✓ Host proxy connection working (Public IP: {})",
                host_output
            );
        } else {
            println!("⚠ Host proxy returned empty response");
        }
    } else {
        println!("⚠ Host proxy connection failed (may be firewall/network issue)");
    }

    // Check for errors in logs
    println!();
    println!("Checking for errors in logs...");
    let error_check = exec.execute_shell("docker exec openvpn-pia cat /var/log/openvpn/openvpn.log 2>/dev/null | tail -50 | grep -iE 'error|failed|frag_in' | tail -5 || echo 'No errors found'")?;
    utils::print_summary(hostname, target_host, all_passed, &error_check.stdout)?;

    if !all_passed {
        anyhow::bail!("VPN verification failed - some tests did not pass");
    }

    Ok(())
}
