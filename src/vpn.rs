use crate::exec::{SshConnection, local};
use anyhow::{Context, Result};
use std::env;
use std::process::Command;

pub fn build_and_push_vpn_image(github_user: &str, image_tag: Option<&str>) -> Result<()> {
    let homelab_dir = crate::config::find_homelab_dir()?;
    let vpn_container_dir = homelab_dir.join("openvpn-container");

    if !vpn_container_dir.exists() {
        anyhow::bail!(
            "VPN container directory not found at {}",
            vpn_container_dir.display()
        );
    }

    // Get git hash for versioning
    let git_hash = local::execute("git", &["rev-parse", "--short", "HEAD"])
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string());

    let base_image = format!("ghcr.io/{}/vpn", github_user);
    let latest_tag = format!("{}:latest", base_image);
    let hash_tag = format!("{}:{}", base_image, git_hash);

    // Use custom tag if provided, otherwise use both latest and hash
    let tags_to_push = if let Some(custom_tag) = image_tag {
        vec![format!("{}:{}", base_image, custom_tag)]
    } else {
        vec![latest_tag.clone(), hash_tag.clone()]
    };

    println!("Building VPN container image...");
    println!("  Tags: {}", tags_to_push.join(", "));
    println!();

    // Build the image with all tags
    let mut build_args = vec!["build"];
    for tag in &tags_to_push {
        build_args.push("-t");
        build_args.push(tag);
    }
    build_args.extend(&["-f", "Dockerfile", "."]);

    let build_status = Command::new("docker")
        .args(&build_args)
        .current_dir(&vpn_container_dir)
        .status()
        .context("Failed to execute docker build")?;

    if !build_status.success() {
        anyhow::bail!("Docker build failed");
    }

    println!("✓ Image built successfully");
    println!();

    // Check if user is logged into GitHub Container Registry
    println!("Checking GitHub Container Registry authentication...");
    let _auth_check = local::execute("docker", &["info"]).context("Failed to check docker info")?;

    // Try to verify we can access ghcr.io
    let login_test = local::execute(
        "docker",
        &["pull", &format!("ghcr.io/{}/vpn:latest", github_user)],
    );

    if let Ok(output) = login_test {
        if !output.status.success() {
            println!("⚠ Warning: Not authenticated or package doesn't exist yet");
            println!("  You may need to login first:");
            println!(
                "  echo $GITHUB_TOKEN | docker login ghcr.io -u {} --password-stdin",
                github_user
            );
            println!();
        }
    }

    println!("Pushing images to GitHub Container Registry...");
    println!();

    // Push all tags
    for tag in &tags_to_push {
        println!("Pushing {}...", tag);
        let push_status = local::execute_status("docker", &["push", tag])
            .with_context(|| format!("Failed to execute docker push for {}", tag))?;

        if !push_status.success() {
            println!();
            println!("❌ Docker push failed for {}", tag);
            println!();
            println!("This usually means:");
            println!("  1. You're not logged into GitHub Container Registry");
            println!("  2. The package doesn't exist yet (first push requires package creation)");
            println!("  3. You don't have write permissions to the repository");
            println!();
            println!("To fix:");
            println!(
                "  1. Create a GitHub Personal Access Token (PAT) with 'write:packages' permission"
            );
            println!("  2. Login to GitHub Container Registry:");
            println!(
                "     echo $GITHUB_TOKEN | docker login ghcr.io -u {} --password-stdin",
                github_user
            );
            println!();
            println!("  3. If this is the first push, make sure the repository exists or");
            println!(
                "     create it at: https://github.com/users/{}/packages/container/vpn",
                github_user
            );
            println!();
            anyhow::bail!("Push failed - see instructions above");
        }
        println!("✓ Pushed {}", tag);
    }

    println!();
    println!("✓ All images pushed successfully");
    println!();
    println!("To use this image, set in your .env file:");
    println!("  VPN_IMAGE={}", latest_tag);
    println!();
    println!("Or update compose/openvpn-pia.docker-compose.yml to use:");
    println!("  image: {}", latest_tag);
    if !git_hash.is_empty() && git_hash != "unknown" {
        println!("  # Or use specific version: image: {}", hash_tag);
    }

    Ok(())
}

pub fn deploy_vpn(hostname: &str, config: &crate::config::EnvConfig) -> Result<()> {
    let homelab_dir = crate::config::find_homelab_dir()?;

    // Load PIA credentials from local .env
    dotenv::from_path(homelab_dir.join(".env")).context("Failed to load .env file")?;

    let pia_username = env::var("PIA_USERNAME").context("PIA_USERNAME not found in .env file")?;
    let pia_password = env::var("PIA_PASSWORD").context("PIA_PASSWORD not found in .env file")?;

    // Get host configuration
    let host_config = config.hosts.get(hostname).with_context(|| {
        format!(
            "Host '{}' not found in .env\n\nAdd configuration to .env:\n  HOST_{}_IP=\"<ip-address>\"\n  HOST_{}_TAILSCALE=\"<tailscale-hostname>\"",
            hostname,
            hostname.to_uppercase(),
            hostname.to_uppercase()
        )
    })?;

    // Determine which host to connect to (prefer IP, fallback to Tailscale)
    let target_host = if let Some(ip) = &host_config.ip {
        ip.clone()
    } else if let Some(tailscale) = &host_config.tailscale {
        tailscale.clone()
    } else {
        anyhow::bail!("No IP or Tailscale hostname configured for {}", hostname);
    };

    println!("Deploying VPN to {} ({})...", hostname, target_host);
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

    // Determine username for SSH and VPN config path
    let default_user = crate::config::get_default_username();
    // Allow VPN_USER to override the username for config path (useful for Portainer)
    // If not set, uses the SSH user (default_user)
    let vpn_user = env::var("VPN_USER").unwrap_or_else(|_| default_user.clone());
    let host_with_user = format!("{}@{}", default_user, target_host);
    let ssh_conn = SshConnection::new(&host_with_user)?;

    // Check if files already exist - if so, skip deployment
    // Check for both .ovpn and .opvn (typo) variants
    // Use /home/$USER/config/vpn (USER can be set via VPN_USER env var)
    let vpn_config_dir = format!("/home/{}/config/vpn", vpn_user);
    let auth_exists = ssh_conn.file_exists(&format!("{}/auth.txt", vpn_config_dir))?;
    let config_exists = ssh_conn.file_exists(&format!("{}/ca-montreal.ovpn", vpn_config_dir))?
        || ssh_conn.file_exists(&format!("{}/ca-montreal.opvn", vpn_config_dir))?;
    let files_exist = auth_exists && config_exists;

    if files_exist {
        println!("✓ VPN configuration files already exist on remote system");
        println!("  Skipping file copy (files are already in place)");
    } else {
        println!("VPN configuration files not found, attempting to copy...");

        // Copy OpenVPN config files to remote system
        let openvpn_dir = homelab_dir.join("openvpn");
        let auth_file = openvpn_dir.join("auth.txt");
        let config_file = openvpn_dir.join("ca-montreal.ovpn");

        if !auth_file.exists() {
            anyhow::bail!("OpenVPN auth file not found at {}", auth_file.display());
        }
        if !config_file.exists() {
            anyhow::bail!("OpenVPN config file not found at {}", config_file.display());
        }

        // Copy files using scp, then move to $HOME/config/vpn
        // Read auth file and write directly
        let auth_content = std::fs::read(&auth_file)
            .with_context(|| format!("Failed to read auth file: {}", auth_file.display()))?;

        // Create directory and write file
        ssh_conn.mkdir_p(&vpn_config_dir)?;
        ssh_conn.write_file(&format!("{}/auth.txt", vpn_config_dir), &auth_content)?;
        ssh_conn.execute_shell_interactive(&format!("chmod 600 {}/auth.txt", vpn_config_dir))?;
        println!("✓ Copied auth.txt to remote system");

        // Copy config file
        let config_content = std::fs::read(&config_file)
            .with_context(|| format!("Failed to read config file: {}", config_file.display()))?;

        ssh_conn.write_file(
            &format!("{}/ca-montreal.ovpn", vpn_config_dir),
            &config_content,
        )?;
        ssh_conn
            .execute_shell_interactive(&format!("chmod 644 {}/ca-montreal.ovpn", vpn_config_dir))?;
        println!("✓ Copied ca-montreal.ovpn to remote system");
    }

    // Copy compose file to remote system (keep in home directory for user access)
    ssh_conn.mkdir_p("$HOME/vpn")?;
    ssh_conn.write_file("$HOME/vpn/docker-compose.yml", compose_content.as_bytes())?;
    println!("✓ Copied VPN compose file to remote system");

    // Create .env file on remote system with PIA credentials
    let env_content = format!(
        "PIA_USERNAME={}\nPIA_PASSWORD={}\n",
        pia_username, pia_password
    );
    ssh_conn.write_file("$HOME/vpn/.env", env_content.as_bytes())?;
    println!("✓ Created .env file on remote system");

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

/// Detect if we're running locally on the target host or remotely
fn is_local_execution(hostname: &str, config: &crate::config::EnvConfig) -> Result<bool> {
    let host_config = config
        .hosts
        .get(hostname)
        .with_context(|| format!("Host '{}' not found in config", hostname))?;

    // Get target IP
    let target_ip = if let Some(ip) = &host_config.ip {
        ip.clone()
    } else {
        // If no IP configured, assume remote
        return Ok(false);
    };

    // Get local IP addresses
    let local_ips = get_local_ips()?;

    // Check if target IP matches any local IP
    Ok(local_ips.contains(&target_ip))
}

/// Get all local IP addresses
fn get_local_ips() -> Result<Vec<String>> {
    let mut ips = Vec::new();

    // Try to get IPs using platform-specific commands
    #[cfg(unix)]
    {
        // Use `hostname -I` on Linux or `ifconfig` on macOS
        if let Ok(output) = local::execute("hostname", &["-I"]) {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for ip in stdout.split_whitespace() {
                ips.push(ip.to_string());
            }
        }

        // Also try `ip addr` on Linux
        if let Ok(output) = local::execute("ip", &["addr", "show"]) {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("inet ") && !line.contains("127.0.0.1") && !line.contains("::1") {
                    if let Some(ip_part) = line.split_whitespace().nth(1) {
                        if let Some(ip) = ip_part.split('/').next() {
                            ips.push(ip.to_string());
                        }
                    }
                }
            }
        }
    }

    #[cfg(windows)]
    {
        // Use `ipconfig` on Windows
        if let Ok(output) = local::execute("ipconfig", &[]) {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("IPv4 Address") || line.contains("IPv4 地址") {
                    if let Some(ip_part) = line.split(':').nth(1) {
                        let ip = ip_part.trim();
                        if !ip.is_empty() {
                            ips.push(ip.to_string());
                        }
                    }
                }
            }
        }
    }

    Ok(ips)
}

pub fn verify_vpn(hostname: &str, config: &crate::config::EnvConfig) -> Result<()> {
    // Get host configuration
    let host_config = config.hosts.get(hostname).with_context(|| {
        format!(
            "Host '{}' not found in .env\n\nAdd configuration to .env:\n  HOST_{}_IP=\"<ip-address>\"\n  HOST_{}_TAILSCALE=\"<tailscale-hostname>\"",
            hostname,
            hostname.to_uppercase(),
            hostname.to_uppercase()
        )
    })?;

    // Determine which host to connect to (prefer IP, fallback to Tailscale)
    let target_host = if let Some(ip) = &host_config.ip {
        ip.clone()
    } else if let Some(tailscale) = &host_config.tailscale {
        tailscale.clone()
    } else {
        anyhow::bail!("No IP or Tailscale hostname configured for {}", hostname);
    };

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  VPN Verification for {}", hostname);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Check if we're running locally or remotely
    let is_local = is_local_execution(hostname, config)?;

    if is_local {
        println!("Detected local execution on {}", hostname);
        println!();
        verify_vpn_local(hostname, &target_host)
    } else {
        println!("Detected remote execution - verifying via SSH");
        println!();
        let default_user = crate::config::get_default_username();
        let host_with_user = format!("{}@{}", default_user, target_host);
        let ssh_conn = SshConnection::new(&host_with_user)?;
        verify_vpn_remote(hostname, &target_host, &ssh_conn)
    }
}

fn print_summary(
    hostname: &str,
    target_host: &str,
    all_passed: bool,
    error_output: &[u8],
) -> Result<()> {
    let error_output_str = String::from_utf8_lossy(error_output);
    if error_output_str.contains("No errors found") || error_output_str.trim().is_empty() {
        println!("   ✓ No recent errors in OpenVPN logs");
    } else {
        println!("   ⚠ Found potential issues in logs:");
        for line in error_output_str.lines().take(5) {
            if !line.trim().is_empty() && !line.contains("No errors found") {
                println!("     - {}", line.trim());
            }
        }
    }

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    if all_passed {
        println!("  Host: {}", hostname);
        println!("  ✓ VPN Verification Complete - All Tests Passed");
    } else {
        println!("  ⚠ VPN Verification Complete - Some Tests Failed");
    }
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!(
        "VPN Status: {}",
        if all_passed {
            "OPERATIONAL"
        } else {
            "ISSUES DETECTED"
        }
    );
    println!();
    println!("Proxy Access:");
    println!("  From host: http://{}:8888", target_host);
    println!("  From containers: http://openvpn-pia:8888");
    println!();
    println!("Example usage:");
    println!(
        "  curl --proxy http://{}:8888 https://api.ipify.org",
        target_host
    );

    Ok(())
}

fn verify_vpn_remote(hostname: &str, target_host: &str, ssh_conn: &SshConnection) -> Result<()> {
    let mut all_passed = true;

    // Test 1: Check if container is running
    println!("[1/10] Checking VPN container status...");
    let container_check = ssh_conn.execute_simple(
        "docker",
        &[
            "ps",
            "--filter",
            "name=openvpn-pia",
            "--format",
            "{{.Names}}",
        ],
    )?;
    let container_output = String::from_utf8_lossy(&container_check.stdout);
    if container_output.trim().contains("openvpn-pia") {
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
    let openvpn_check = ssh_conn.execute_shell("docker exec openvpn-pia pgrep -f openvpn")?;
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
    let tun_check = ssh_conn.execute_shell("docker exec openvpn-pia ip link show tun0 2>&1")?;
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
            println!("   ⚠ TUN interface exists but no IP found");
        }
    } else {
        println!("   ✗ TUN interface not found");
        all_passed = false;
    }

    // Test 4: Check routing
    println!("[4/10] Checking routing configuration...");
    let route_check = ssh_conn.execute_shell(
        "docker exec openvpn-pia ip route | grep -E '0\\.0\\.0\\.0/1|128\\.0\\.0\\.0/1'",
    )?;
    if route_check.status.success() {
        let route_output = String::from_utf8_lossy(&route_check.stdout);
        if route_output.contains("tun0") {
            println!("   ✓ Traffic is routed through VPN");
        } else {
            println!("   ⚠ Warning: Routes may not be configured correctly");
            all_passed = false;
        }
    } else {
        println!("   ⚠ Warning: Could not verify routing");
    }

    // Test 5: Check Privoxy
    println!("[5/10] Checking Privoxy process...");
    let privoxy_check = ssh_conn.execute_shell("docker exec openvpn-pia pgrep privoxy")?;
    if privoxy_check.status.success() {
        let pid = String::from_utf8_lossy(&privoxy_check.stdout)
            .trim()
            .to_string();
        println!("   ✓ Privoxy is running (PID: {})", pid);
    } else {
        println!("   ✗ Privoxy is not running");
        all_passed = false;
    }

    // Test 6: Check Privoxy port
    println!("[6/10] Checking Privoxy port 8888...");
    let port_check = ssh_conn.execute_shell("docker exec openvpn-pia ss -tlnp 2>/dev/null | grep 8888 || docker exec openvpn-pia netstat -tlnp 2>/dev/null | grep 8888")?;
    if port_check.status.success() {
        println!("   ✓ Privoxy is listening on port 8888");
    } else {
        println!("   ✗ Privoxy port 8888 not found");
        println!("    Host: {}", hostname);
        all_passed = false;
    }

    // Test 7: Test DNS resolution
    println!("[7/10] Testing DNS resolution...");
    let dns_check =
        ssh_conn.execute_shell("docker exec openvpn-pia nslookup api.ipify.org 2>&1 | head -5")?;
    if dns_check.status.success() {
        let dns_output = String::from_utf8_lossy(&dns_check.stdout);
        if dns_output.contains("Name:") || dns_output.contains("Address:") {
            println!("   ✓ DNS resolution working");
        } else {
            println!("   ⚠ DNS resolution may have issues");
        }
    } else {
        println!("   ⚠ DNS resolution test failed");
    }

    // Test 8: Test direct connectivity (should show VPN IP)
    println!("[8/10] Testing direct connectivity (should show VPN IP)...");
    let direct_ip = ssh_conn
        .execute_shell("docker exec openvpn-pia curl -s --max-time 10 https://api.ipify.org")?;
    if direct_ip.status.success() {
        let ip_output = String::from_utf8_lossy(&direct_ip.stdout)
            .trim()
            .to_string();
        if !ip_output.is_empty() {
            println!("   ✓ Direct connection working (Public IP: {})", ip_output);
        } else {
            println!("   ✗ Direct connection returned empty response");
            all_passed = false;
        }
    } else {
        println!("   ✗ Direct connection failed");
        all_passed = false;
    }

    // Test 9: Test proxy connectivity
    println!("[9/10] Testing proxy connectivity...");
    let proxy_ip = ssh_conn.execute_shell("docker exec openvpn-pia curl -s --proxy http://127.0.0.1:8888 --max-time 10 https://api.ipify.org")?;
    if proxy_ip.status.success() {
        let proxy_output = String::from_utf8_lossy(&proxy_ip.stdout).trim().to_string();
        if !proxy_output.is_empty() {
            println!(
                "   ✓ Proxy connection working (Public IP: {})",
                proxy_output
            );
        } else {
            println!("   ✗ Proxy connection returned empty response");
            all_passed = false;
        }
    } else {
        println!("   ✗ Proxy connection failed");
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
            println!("   ⚠ Host proxy returned empty response");
        }
    } else {
        println!("   ⚠ Host proxy connection failed (may be firewall/network issue)");
    }

    // Check for errors in logs
    println!();
    println!("Checking for errors in logs...");
    let error_check = ssh_conn.execute_shell("docker exec openvpn-pia cat /var/log/openvpn/openvpn.log 2>/dev/null | tail -50 | grep -iE 'error|failed|frag_in' | tail -5 || echo 'No errors found'")?;
    print_summary(hostname, target_host, all_passed, &error_check.stdout)?;

    if !all_passed {
        anyhow::bail!("VPN verification failed - some tests did not pass");
    }

    Ok(())
}

fn verify_vpn_local(hostname: &str, target_host: &str) -> Result<()> {
    let mut all_passed = true;

    // Test 1: Check if container is running
    println!("[1/10] Checking VPN container status...");
    let container_check = local::execute(
        "docker",
        &[
            "ps",
            "--filter",
            "name=openvpn-pia",
            "--format",
            "{{.Names}}",
        ],
    )?;
    let container_output = String::from_utf8_lossy(&container_check.stdout);
    if container_output.trim().contains("openvpn-pia") {
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
    let openvpn_check = local::execute_shell("docker exec openvpn-pia pgrep -f openvpn")?;
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
    let tun_check = local::execute_shell("docker exec openvpn-pia ip link show tun0 2>&1")?;
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
            println!("   ⚠ TUN interface exists but no IP found");
        }
    } else {
        println!("   ✗ TUN interface not found");
        all_passed = false;
    }

    // Test 4: Check routing
    println!("[4/10] Checking routing configuration...");
    let route_check = local::execute_shell(
        "docker exec openvpn-pia ip route | grep -E '0\\.0\\.0\\.0/1|128\\.0\\.0\\.0/1'",
    )?;
    if route_check.status.success() {
        let route_output = String::from_utf8_lossy(&route_check.stdout);
        if route_output.contains("tun0") {
            println!("   ✓ Traffic is routed through VPN");
        } else {
            println!("   ⚠ Warning: Routes may not be configured correctly");
            all_passed = false;
        }
    } else {
        println!("   ⚠ Warning: Could not verify routing");
    }

    // Test 5: Check Privoxy
    println!("[5/10] Checking Privoxy process...");
    let privoxy_check = local::execute_shell("docker exec openvpn-pia pgrep privoxy")?;
    if privoxy_check.status.success() {
        let pid = String::from_utf8_lossy(&privoxy_check.stdout)
            .trim()
            .to_string();
        println!("   ✓ Privoxy is running (PID: {})", pid);
    } else {
        println!("   ✗ Privoxy is not running");
        all_passed = false;
    }

    // Test 6: Check Privoxy port
    println!("[6/10] Checking Privoxy port 8888...");
    let port_check = local::execute_shell(
        "docker exec openvpn-pia ss -tlnp 2>/dev/null | grep 8888 || docker exec openvpn-pia netstat -tlnp 2>/dev/null | grep 8888",
    )?;
    if port_check.status.success() {
        println!("   ✓ Privoxy is listening on port 8888");
    } else {
        println!("   ✗ Privoxy port 8888 not found");
        all_passed = false;
    }

    // Test 7: Test DNS resolution
    println!("[7/10] Testing DNS resolution...");
    let dns_check =
        local::execute_shell("docker exec openvpn-pia nslookup api.ipify.org 2>&1 | head -5")?;
    if dns_check.status.success() {
        let dns_output = String::from_utf8_lossy(&dns_check.stdout);
        if dns_output.contains("Name:") || dns_output.contains("Address:") {
            println!("   ✓ DNS resolution working");
        } else {
            println!("   ⚠ DNS resolution may have issues");
        }
    } else {
        println!("   ⚠ DNS resolution test failed");
    }

    // Test 8: Test direct connectivity (should show VPN IP)
    println!("[8/10] Testing direct connectivity (should show VPN IP)...");
    let direct_ip = local::execute_shell(
        "docker exec openvpn-pia curl -s --max-time 10 https://api.ipify.org",
    )?;
    if direct_ip.status.success() {
        let ip_output = String::from_utf8_lossy(&direct_ip.stdout)
            .trim()
            .to_string();
        if !ip_output.is_empty() {
            println!("   ✓ Direct connection working (Public IP: {})", ip_output);
        } else {
            println!("   ✗ Direct connection returned empty response");
            all_passed = false;
        }
    } else {
        println!("   ✗ Direct connection failed");
        all_passed = false;
    }

    // Test 9: Test proxy connectivity
    println!("[9/10] Testing proxy connectivity...");
    let proxy_ip = local::execute_shell(
        "docker exec openvpn-pia curl -s --proxy http://127.0.0.1:8888 --max-time 10 https://api.ipify.org",
    )?;
    if proxy_ip.status.success() {
        let proxy_output = String::from_utf8_lossy(&proxy_ip.stdout).trim().to_string();
        if !proxy_output.is_empty() {
            println!(
                "   ✓ Proxy connection working (Public IP: {})",
                proxy_output
            );
        } else {
            println!("   ✗ Proxy connection returned empty response");
            all_passed = false;
        }
    } else {
        println!("   ✗ Proxy connection failed");
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
            println!("   ⚠ Host proxy returned empty response");
        }
    } else {
        println!("   ⚠ Host proxy connection failed (may be firewall/network issue)");
    }

    // Check for errors in logs
    println!();
    println!("Checking for errors in logs...");
    let error_check = local::execute_shell(
        "docker exec openvpn-pia cat /var/log/openvpn/openvpn.log 2>/dev/null | tail -50 | grep -iE 'error|failed|frag_in' | tail -5 || echo 'No errors found'",
    )?;
    print_summary(hostname, target_host, all_passed, &error_check.stdout)?;

    if !all_passed {
        anyhow::bail!("VPN verification failed - some tests did not pass");
    }

    Ok(())
}
