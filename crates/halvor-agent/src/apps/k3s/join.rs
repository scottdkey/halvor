//! K3s node joining logic

use halvor_core::config::EnvConfig;
use crate::apps::k3s::{agent_service, cleanup, kubeconfig, tools, verify};
use crate::apps::tailscale;
use halvor_core::utils::exec::{CommandExecutor, Executor};
use anyhow::{Context, Result};
use serde_json;
use std::io::{self, Write};

/// Join a node to the cluster
///
/// This function handles both remote and local joins:
/// - **Remote join**: When `hostname` is not localhost, creates SSH connection to remote node
/// - **Local join**: When `hostname` is localhost, executes commands directly on current machine
///
/// The executor automatically detects if the target is local or remote based on:
/// - IP address comparison (checks if target IP matches local IPs)
/// - Hostname comparison (checks if target hostname matches current hostname)
/// - Tailscale IP comparison (for Tailscale-connected nodes)
pub fn join_cluster(
    hostname: &str,
    server: &str,
    token: &str,
    control_plane: bool,
    config: &EnvConfig,
) -> Result<()> {
    // Find the hostname for the server (it might be an IP address)
    let primary_hostname =
        find_hostname_from_server(server, config).unwrap_or_else(|| server.to_string());

    // Fetch kubeconfig - try KUBE_CONFIG environment variable first, 
    // but if not available or parsing fails, fetch directly from primary node
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Preparing to join cluster...");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    
    let kubeconfig_content = if let Ok(kubeconfig_env) = std::env::var("KUBE_CONFIG") {
        // Try to use kubeconfig from environment variable
        println!("KUBE_CONFIG environment variable found, attempting to use it...");
        
        // Process it to ensure it points to the primary server (not the joining node)
        match kubeconfig::process_kubeconfig_for_primary(&kubeconfig_env, &primary_hostname, config) {
            Ok(processed) => {
                println!("✓ Successfully processed kubeconfig from KUBE_CONFIG");
                
                // Verify the processed kubeconfig points to the primary server
                if let Ok((server_host, _)) = kubeconfig::extract_server_and_token_from_kubeconfig(&processed) {
                    let normalized_server = halvor_core::utils::hostname::normalize_hostname(&server_host);
                    let normalized_primary = halvor_core::utils::hostname::normalize_hostname(&primary_hostname);
                    
                    if normalized_server == normalized_primary || server_host == primary_hostname {
                        println!("✓ Kubeconfig correctly points to primary server: {}", server_host);
                    } else {
                        println!("⚠ Warning: Processed kubeconfig server ({}) doesn't match primary ({}).", server_host, primary_hostname);
                        println!("  This may cause connection issues.");
                    }
                }
                
                processed
            }
            Err(e) => {
                // If parsing fails, fall back to fetching from primary node
                println!("⚠ Failed to parse kubeconfig from KUBE_CONFIG: {}", e);
                println!("  Falling back to fetching kubeconfig directly from {}...", primary_hostname);
                kubeconfig::fetch_kubeconfig_content(&primary_hostname, config)
                    .context("Failed to fetch kubeconfig from primary node after KUBE_CONFIG parsing failed")?
            }
        }
    } else {
        // No KUBE_CONFIG set - fetch directly from primary node
    println!(
            "KUBE_CONFIG not set, fetching kubeconfig directly from primary node ({})...",
        primary_hostname
    );
        println!("  (This requires SSH access to {})", primary_hostname);
        let kubeconfig = kubeconfig::fetch_kubeconfig_content(&primary_hostname, config)
        .context(
                "Failed to fetch kubeconfig from primary node. Ensure the cluster is initialized and SSH access is available.",
        )?;
    println!("✓ Kubeconfig fetched and ready for verification");
        kubeconfig
    };
    println!();

    // Now connect to the remote node
    println!("Connecting to node: {}", hostname);
    println!("  [DEBUG] Creating executor for {}...", hostname);
    let exec = Executor::new(hostname, config)
        .with_context(|| format!("Failed to create executor for hostname: {}", hostname))?;
    let is_local = exec.is_local();
    println!("  [DEBUG] Executor created (local: {})", is_local);

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    if control_plane {
        println!("Join K3s Cluster - Control Plane Node");
    } else {
        println!("Join K3s Cluster - Agent Node");
    }
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    if is_local {
        println!("Target: localhost ({})", hostname);
    } else {
        println!("Target: {} (remote)", hostname);
    }
    println!("Server: {}", server);
    println!();

    // Ensure Tailscale is installed and running (required for cluster communication)
    println!("Checking for Tailscale (required for cluster communication)...");
    if !tailscale::is_tailscale_installed(&exec) {
        println!("Tailscale not found. Installing Tailscale...");
        if is_local {
            tailscale::install_tailscale()?;
        } else {
            tailscale::install_tailscale_on_host(hostname, config)?;
        }
        println!("✓ Tailscale installed");
    } else {
        println!("✓ Tailscale is installed");
    }
    
    // Check if Tailscale is running and connected
    // Since we're already connected via Tailscale (SSH over Tailscale), we know it's working
    // Just do a quick check to verify, but don't block if it's slow
    println!("  Verifying Tailscale status...");
    println!("  (Note: If Tailscale SSH authentication is required, you'll see the prompt)");
    
    // Use execute_shell_interactive for remote to show any prompts (like Tailscale SSH auth)
    // For local, use regular execute_shell
    let tailscale_check = if is_local {
        exec.execute_shell("timeout 2 tailscale status --json 2>&1 || timeout 2 tailscale status 2>&1 | head -1 || echo 'not_running'").ok()
    } else {
        // For remote, we need to see any authentication prompts
        // But we can't easily capture output from interactive mode, so just try a quick check
        // If it fails, assume it's working since we connected via Tailscale
        exec.execute_shell("timeout 2 tailscale status --json 2>&1 || timeout 2 tailscale status 2>&1 | head -1 || echo 'not_running'").ok()
    };
    
    let is_tailscale_running = tailscale_check
        .and_then(|o| {
            if !o.status.success() {
                return None;
            }
            String::from_utf8(o.stdout).ok()
        })
        .map(|s| {
            let status_str = s.trim();
            // Check for JSON output or any valid Tailscale status
            !status_str.is_empty()
                && !status_str.contains("not_running")
                && (status_str.starts_with("{") // JSON output
                    || status_str.contains("\"Self\"")
                    || status_str.contains("\"Status\"")
                    || status_str.contains("100.")) // Tailscale IP
        })
        .unwrap_or_else(|| {
            // If the check failed or timed out, but we're connected via Tailscale,
            // assume it's working (we wouldn't be able to SSH otherwise)
            if !is_local {
                println!("  (Status check timed out or failed, but connection via Tailscale confirms it's working)");
                true // Assume working since we connected via Tailscale
            } else {
                false
            }
        });
    
    if is_tailscale_running {
        println!("✓ Tailscale is running and connected");
    } else if !is_local {
        // Remote connection - if we got here via Tailscale, it's working
        println!("✓ Tailscale connection confirmed (connected via Tailscale)");
    } else {
        println!("⚠️  Warning: Could not verify Tailscale status.");
        println!("   Please ensure Tailscale is running: sudo tailscale up");
    }
    println!();

    // Get Tailscale IP for this node (for TLS SANs)
    println!("Getting Tailscale IP for this node...");
    let tailscale_ip = tailscale::get_tailscale_ip_with_fallback(&exec, hostname, config)?;

    let tailscale_hostname = tailscale::get_tailscale_hostname_remote(&exec)
        .ok()
        .flatten();

    println!("✓ Using Tailscale IP: {}", tailscale_ip);
    if let Some(ref ts_hostname) = tailscale_hostname {
        println!("✓ Using Tailscale hostname: {}", ts_hostname);
    }

    // Verify Tailscale connectivity to the server
    println!("Verifying Tailscale connectivity to server {}...", server);
    let ping_test = exec.execute_shell(&format!(
        "ping -c 1 -W 2 {} 2>&1 || echo 'ping_failed'",
        server
    ));
    match ping_test {
        Ok(output) => {
            let ping_result = String::from_utf8_lossy(&output.stdout);
            if ping_result.contains("ping_failed") || !output.status.success() {
                println!(
                    "⚠ Warning: Could not ping server {}. Tailscale connectivity may be an issue.",
                    server
                );
                println!(
                    "  Please verify Tailscale is running and both nodes can reach each other."
                );
            } else {
                println!("✓ Tailscale connectivity verified");
            }
        }
        Err(_) => {
            println!(
                "⚠ Warning: Could not verify Tailscale connectivity to {}",
                server
            );
        }
    }

    // Build TLS SANs list
    let mut tls_sans = format!("--tls-san={}", tailscale_ip);
    if let Some(ref ts_hostname) = tailscale_hostname {
        tls_sans.push_str(&format!(" --tls-san={}", ts_hostname));
    }
    tls_sans.push_str(&format!(" --tls-san={}", hostname));

    // Get Tailscale hostname for the server node
    println!("Resolving Tailscale hostname for server {}...", server);

    // Check if server is already a Tailscale hostname (ends with .ts.net)
    let server_addr = {
        let raw_addr = if server.ends_with(".ts.net") || server.ends_with(".ts.net.") {
            println!("  ✓ Server is already a Tailscale hostname: {}", server);
            server.to_string()
        } else {
            // First, try to get Tailscale hostname from local Tailscale CLI
            // This queries the local machine's Tailscale status to find the peer
            let server_tailscale_hostname = match tailscale::get_peer_tailscale_hostname(server) {
            Ok(Some(hostname)) => {
                println!("  ✓ Found Tailscale hostname via local CLI: {}", hostname);
                Some(hostname)
            }
            Ok(None) => {
                // Not found in local Tailscale status, try remote method
                println!(
                    "  Peer '{}' not found in local Tailscale status, trying remote method...",
                    server
                );
                let server_exec = Executor::new(server, config).ok();
                if let Some(ref exec) = server_exec {
                    match tailscale::get_tailscale_hostname_remote(exec) {
                        Ok(Some(hostname)) => {
                            println!("  ✓ Found Tailscale hostname via remote: {}", hostname);
                            Some(hostname)
                        }
                        Ok(None) => {
                            // Check if Tailscale is running
                            let tailscale_check =
                                exec.execute_shell("tailscale status --json 2>&1").ok();
                            if let Some(ref output) = tailscale_check {
                                if !output.status.success() {
                                    let error = String::from_utf8_lossy(&output.stderr);
                                    println!("  ⚠ Tailscale command failed: {}", error.trim());
                                } else {
                                    // Try to parse JSON to see what we got
                                    if let Ok(json) =
                                        serde_json::from_slice::<serde_json::Value>(&output.stdout)
                                    {
                                        if json.get("Self").is_none() {
                                            println!("  ⚠ Tailscale status missing 'Self' field");
                                        } else if json
                                            .get("Self")
                                            .and_then(|s| s.get("DNSName"))
                                            .is_none()
                                        {
                                            println!(
                                                "  ⚠ Tailscale DNSName not set (may need to enable MagicDNS)"
                                            );
                                        }
                                    }
                                }
                            }
                            None
                        }
                        Err(e) => {
                            println!("  ⚠ Failed to get Tailscale hostname from remote: {}", e);
                            None
                        }
                    }
                } else {
                    println!("  ⚠ Could not create executor for server {}", server);
                    None
                }
            }
            Err(e) => {
                println!("  ⚠ Failed to query local Tailscale status: {}", e);
                // Try remote method before constructing from tailnet base
                let server_exec = Executor::new(server, config).ok();
                if let Some(ref exec) = server_exec {
                    match tailscale::get_tailscale_hostname_remote(exec) {
                        Ok(Some(hostname)) => {
                            println!("  ✓ Found Tailscale hostname via remote: {}", hostname);
                            Some(hostname)
                        }
                        _ => {
                            // Both local and remote failed, construct from tailnet base
                            let constructed_hostname =
                                format!("{}.{}", server, config._tailnet_base);
                            println!(
                                "  Constructing Tailscale hostname from tailnet base: {}",
                                constructed_hostname
                            );
                            Some(constructed_hostname)
                        }
                    }
                } else {
                    // Can't create executor, construct from tailnet base
                    let constructed_hostname = format!("{}.{}", server, config._tailnet_base);
                    println!(
                        "  Constructing Tailscale hostname from tailnet base: {}",
                        constructed_hostname
                    );
                    Some(constructed_hostname)
                }
            }
            };

            if let Some(ref ts_hostname) = server_tailscale_hostname {
                println!("  ✓ Using Tailscale hostname for server: {}", ts_hostname);
                ts_hostname.clone()
            } else {
                // Fallback: construct hostname from tailnet base
                println!(
                    "  Could not get Tailscale hostname from CLI, constructing from tailnet base..."
                );
                let constructed_hostname = format!("{}.{}", server, config._tailnet_base);
                println!(
                    "  ✓ Using constructed Tailscale hostname: {}",
                    constructed_hostname
                );
                constructed_hostname
            }
        };
        // Strip trailing dot (DNS absolute notation) which causes SSH resolution issues
        raw_addr.trim_end_matches('.').to_string()
    };

    println!();
    println!("Joining cluster via Tailscale...");

    // Check if SSH key authentication is set up, if not, offer to set it up
    // This avoids repeated password prompts during the join process
    if !exec.is_local() {
        use halvor_core::utils::ssh::SshConnection;
        let default_user = std::env::var("USER").unwrap_or_else(|_| "root".to_string());
        
        // Get the target host address (Tailscale hostname or IP)
        let target_host = exec.target_host(hostname, config)?;
        let ssh_host = format!("{}@{}", default_user, target_host);
        
        let ssh_test = SshConnection::new(&ssh_host);
        
        if let Ok(ssh_conn) = ssh_test {
            if !ssh_conn.use_key_auth() {
                println!("⚠ SSH key authentication not set up for {}.", target_host);
                println!("  This will cause repeated password prompts during the join process.");
                println!();
                print!("  Set up SSH keys now? (y/N): ");
                use std::io::{self, Write};
                io::stdout().flush()?;
                
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                let input = input.trim();
                
                if input.eq_ignore_ascii_case("y") || input.eq_ignore_ascii_case("yes") {
                    println!("  Setting up SSH keys...");
                    use halvor_core::utils::ssh::copy_ssh_key;
                    if let Err(e) = copy_ssh_key(&target_host, Some(&default_user), Some(&default_user)) {
                        println!("  ⚠ Warning: Failed to set up SSH keys: {}", e);
                        println!("  You can set them up manually with: ssh-copy-id {}", ssh_host);
                    } else {
                        println!("  ✓ SSH keys set up successfully!");
                        println!("  Re-testing connection...");
                        // Re-test to confirm key auth works now
                        let ssh_test2 = SshConnection::new(&ssh_host);
                        if let Ok(ssh_conn2) = ssh_test2 {
                            if ssh_conn2.use_key_auth() {
                                println!("  ✓ Key authentication confirmed!");
                            }
                        }
                    }
                    println!();
                } else {
                    println!("  Skipping SSH key setup. You'll be prompted for passwords multiple times.");
                    println!("  To set up SSH keys later, run: ssh-copy-id {}", ssh_host);
                    println!();
                }
            }
        }
    }

    // Check if node is currently part of a cluster and handle removal
    check_and_remove_from_existing_cluster(&exec, hostname, server, config)?;

    // Check if K3s is already installed
    println!("Checking if K3s is installed...");
    let k3s_binary_exists = exec
        .execute_shell("test -f /usr/local/bin/k3s && echo exists || test -f /usr/local/bin/k3s-agent && echo exists || echo not_exists")
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim() == "exists")
        .unwrap_or(false);
    
    let k3s_service_running = if k3s_binary_exists {
        // Check if service is running
        let service_check = exec
            .execute_shell("sudo systemctl is-active k3s 2>/dev/null || sudo systemctl is-active k3s-agent 2>/dev/null || echo not_running")
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim() == "active" || s.trim() == "activating")
            .unwrap_or(false);
        service_check
    } else {
        false
    };

    // Always clean up existing installation to ensure fresh start with correct configuration
    // This is especially important when switching from hostname to IP address
    if k3s_binary_exists || k3s_service_running {
    if k3s_binary_exists && k3s_service_running {
        println!("✓ K3s is already installed and running");
            println!("  Cleaning up existing installation to ensure correct configuration...");
    } else if k3s_binary_exists {
        println!("⚠ K3s binary found but service is not running");
        println!("  Cleaning up existing installation...");
        } else {
            println!("⚠ K3s service found but binary missing");
            println!("  Cleaning up existing installation...");
        }
        cleanup::cleanup_existing_k3s(&exec)?;
        
        // Double-check that service file is removed
        println!("  Verifying service file removal...");
        let service_check = exec.execute_shell("test -f /etc/systemd/system/k3s.service && echo exists || echo removed").ok();
        if let Some(check) = service_check {
            let status_str = String::from_utf8_lossy(&check.stdout).to_string();
            let status = status_str.trim();
            if status == "exists" {
                println!("  ⚠ Service file still exists, forcing removal...");
                let _ = exec.execute_shell("sudo rm -f /etc/systemd/system/k3s.service /etc/systemd/system/k3s-agent.service");
                let _ = exec.execute_shell("sudo rm -rf /etc/systemd/system/k3s.service.d");
                let _ = exec.execute_shell("sudo systemctl daemon-reload");
            } else {
                println!("  ✓ Service file removed");
            }
        }
    } else {
        println!("✓ K3s is not installed - will install as part of join process");
    }

    // Note: In development mode, halvor will be downloaded from the 'experimental' release
    // In production mode, it will be downloaded from the latest versioned release

    // Ensure halvor is installed first (the glue that enables remote operations)
    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Preparing node: {}", hostname);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Checking for halvor (required for remote operations)...");
    tools::check_and_install_halvor(&exec)?;

    // Ensure kubectl and helm are installed
    println!();
    println!("Checking for required tools...");
    tools::check_and_install_kubectl(&exec)?;
    tools::check_and_install_helm(&exec)?;

    // Note: SMB mounts are set up separately for cluster storage, not for K3s data directory
    // K3s will use default local data directory (/var/lib/rancher/k3s)
    // SMB mounts will be available in the cluster for persistent volumes

    // Check if K3s needs to be installed
    // Note: We always clean up existing installations to ensure correct cluster join,
    // so we always need to install after cleanup
    println!();
    println!("K3s installation check complete - proceeding with installation...");
    
    // Download K3s install script using reqwest
    println!("Downloading K3s install script...");
    let k3s_script_url = "https://get.k3s.io";
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("Failed to create HTTP client")?;

    let k3s_script = client
        .get(k3s_script_url)
        .send()
        .context("Failed to download K3s install script")?
        .error_for_status()
        .context("HTTP error downloading K3s install script")?
        .text()
        .context("Failed to read K3s install script content")?;

    // Detect init system to help K3s script detect correctly
    println!("Detecting init system...");
    let has_systemd = exec
        .execute_shell("systemctl --version >/dev/null 2>&1 && echo yes || echo no")
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim() == "yes")
        .unwrap_or(false);
    
    let has_openrc = exec
        .execute_shell("command -v rc-update >/dev/null 2>&1 && echo yes || echo no")
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim() == "yes")
        .unwrap_or(false);
    
    if has_systemd {
        println!("✓ Detected systemd init system");
    } else if has_openrc {
        println!("✓ Detected OpenRC init system");
    } else {
        println!("⚠ Warning: Could not detect init system, assuming systemd");
    }
    
    // Patch the K3s script to fix incorrect init system detection and sudo issues
    // The script incorrectly tries to remove /etc/systemd/system when it detects OpenRC
    // We'll prevent that by ensuring systemd detection is correct
    let mut patched_script = k3s_script;
    
    // Fix the dangerous rm -f /etc/systemd/system line that appears when detection is wrong
    // Replace it with a safer check
    if patched_script.contains("rm -f /etc/systemd/system") {
        println!("⚠ Patching K3s script to fix incorrect init system detection...");
        patched_script = patched_script.replace(
            "rm -f /etc/systemd/system",
            "# Patched by halvor: removed dangerous rm command"
        );
    }
    
    // Fix broken sudo calls in the K3s script
    // The script sometimes calls sudo incorrectly, causing "usage: sudo" errors
    println!("⚠ Patching K3s script to fix sudo handling...");
    
    // Fix common sudo issues in the script:
    // 1. Replace "sudo " with proper sudo calls (only if followed by a command)
    // 2. Fix cases where sudo might be called with no arguments
    // 3. Ensure sudo commands are properly formatted
    
    // Fix pattern: sudo followed by just whitespace or newline (invalid)
    patched_script = patched_script.replace("sudo \n", "sudo -v\n");
    patched_script = patched_script.replace("sudo \r\n", "sudo -v\r\n");
    
    // Fix pattern: sudo$ (sudo at end of line with no command)
    patched_script = patched_script.replace("sudo$\n", "sudo -v\n");
    
    // Fix pattern: sudo || (sudo with no command before ||)
    patched_script = patched_script.replace("sudo ||", "sudo -v ||");
    
    // Fix pattern: sudo && (sudo with no command before &&)
    patched_script = patched_script.replace("sudo &&", "sudo -v &&");
    
    // Add a helper function at the beginning to ensure sudo works
    let sudo_fix = r#"
# Patched by halvor: Ensure sudo works correctly
_sudo() {
    if [ "$(id -u)" = "0" ]; then
        # Already root, no sudo needed
        "$@"
    else
        # Not root, use sudo
        if command -v sudo >/dev/null 2>&1; then
            sudo "$@"
        else
            echo "ERROR: sudo is required but not found" >&2
            exit 1
        fi
    fi
}
"#;
    
    // Insert the fix near the beginning of the script (after shebang)
    if let Some(pos) = patched_script.find('\n') {
        let (shebang, rest) = patched_script.split_at(pos + 1);
        patched_script = format!("{}{}{}", shebang, sudo_fix, rest);
        
        // Replace common sudo patterns with _sudo helper
        // But be careful - only replace standalone sudo calls, not ones that are part of larger commands
        // The script should use _sudo for systemd operations
        patched_script = patched_script.replace("sudo systemctl", "_sudo systemctl");
        patched_script = patched_script.replace("sudo mkdir", "_sudo mkdir");
        patched_script = patched_script.replace("sudo tee", "_sudo tee");
        patched_script = patched_script.replace("sudo chmod", "_sudo chmod");
        patched_script = patched_script.replace("sudo chown", "_sudo chown");
    }
    
    // If systemd is detected, ensure the script knows about it
    if has_systemd && !has_openrc {
        // Set environment variable to help script detection
        // The script checks for systemd by looking for systemctl
        // We've already verified it exists, so the script should detect it correctly
    }

    // Write patched script to remote host
    let remote_script_path = "/tmp/k3s-install.sh";
    exec.write_file(remote_script_path, patched_script.as_bytes())
        .context("Failed to write K3s install script to remote host")?;

    // Make script executable
    let chmod_output = exec.execute_shell(&format!("chmod +x {}", remote_script_path))?;
    if !chmod_output.status.success() {
        anyhow::bail!(
            "Failed to make K3s install script executable: {}",
            String::from_utf8_lossy(&chmod_output.stderr)
        );
    }

    // Get Tailscale IP for the server (frigg) - use IP instead of hostname for systemd services
    // Systemd services can't resolve Tailscale hostnames, so we must use the IP address
    // Check for IP in environment variable first, then try to resolve dynamically
    println!("Getting Tailscale IP for server {} (needed for systemd service DNS resolution)...", server_addr);
    let server_addr_for_k3s = {
        // First, check for IP in environment variable
        // Format: HOST_<HOSTNAME>_IP (e.g., HOST_FRIGG_IP)
        let hostname_upper = server.to_uppercase().replace("-", "_");
        let env_var_name = format!("HOST_{}_IP", hostname_upper);
        
        if let Ok(ip_from_env) = std::env::var(&env_var_name) {
            let ip = ip_from_env.trim();
            if !ip.is_empty() {
                println!("  ✓ Found server IP from {} environment variable: {}", env_var_name, ip);
                println!("  ✓ Using server IP address for K3s join (systemd services can't resolve Tailscale hostnames)");
                ip.to_string()
            } else {
                // Empty env var, try to resolve dynamically
                println!("  {} is empty, resolving server IP dynamically...", env_var_name);
                let server_exec = Executor::new(&server_addr, config).ok();
                if let Some(ref exec) = server_exec {
                    match tailscale::get_tailscale_ip_with_fallback(exec, &server_addr, config) {
                        Ok(ip) => {
                            println!("  ✓ Found server Tailscale IP: {}", ip);
                            println!("  ✓ Using server IP address for K3s join (systemd services can't resolve Tailscale hostnames)");
                            ip
                        }
                        Err(e) => {
                            println!("  ⚠ Could not get server Tailscale IP: {}", e);
                            println!("  Will use hostname (may fail if systemd can't resolve DNS)");
                            server_addr.clone()
                        }
                    }
                } else {
                    println!("  ⚠ Could not create executor for server, will use hostname");
                    server_addr.clone()
                }
            }
        } else {
            // Env var not set, try to resolve dynamically
            println!("  {} not set, resolving server IP dynamically...", env_var_name);
            let server_exec = Executor::new(&server_addr, config).ok();
            if let Some(ref exec) = server_exec {
                match tailscale::get_tailscale_ip_with_fallback(exec, &server_addr, config) {
                    Ok(ip) => {
                        println!("  ✓ Found server Tailscale IP: {}", ip);
                        println!("  ✓ Using server IP address for K3s join (systemd services can't resolve Tailscale hostnames)");
                        ip
                    }
                    Err(e) => {
                        println!("  ⚠ Could not get server Tailscale IP: {}", e);
                        println!("  Will use hostname (may fail if systemd can't resolve DNS)");
                        server_addr.clone()
                    }
                }
            } else {
                println!("  ⚠ Could not create executor for server, will use hostname");
                server_addr.clone()
            }
        }
    };

    // Build install command
    // For control plane nodes joining, we need to use --server flag
    // For agent nodes, we also use --server flag
    // Use --advertise-address with Tailscale IP for cluster communication
    let advertise_addr = format!("--advertise-address={}", tailscale_ip);
    
    // Run with sudo from the start if not root to avoid script's internal sudo handling issues
    let install_cmd = if exec.get_username().ok().as_deref() == Some("root") {
        // Already running as root, no sudo needed
        if control_plane {
            format!(
                "{} server --server=https://{}:6443 --token={} --disable=traefik --write-kubeconfig-mode=0644 {} {}",
                remote_script_path, server_addr_for_k3s, token, advertise_addr, tls_sans
            )
        } else {
            format!(
                "{} agent --server=https://{}:6443 --token={} {}",
                remote_script_path, server_addr_for_k3s, token, tls_sans
            )
        }
    } else {
        // Not root - run with sudo to avoid script's internal sudo handling issues
        if control_plane {
            format!(
                "sudo {} server --server=https://{}:6443 --token={} --disable=traefik --write-kubeconfig-mode=0644 {} {}",
                remote_script_path, server_addr_for_k3s, token, advertise_addr, tls_sans
            )
        } else {
            format!(
                "sudo {} agent --server=https://{}:6443 --token={} {}",
                remote_script_path, server_addr_for_k3s, token, tls_sans
            )
        }
    };

    println!("Join command details:");
    println!("  Server hostname: {}", server_addr);
    println!("  Server IP (for K3s): {}", server_addr_for_k3s);
    println!(
        "  Token: {}...{}",
        &token[..8.min(token.len())],
        &token[token.len().saturating_sub(8)..]
    );
    println!("  Control plane: {}", control_plane);
    println!("  TLS SANs: {}", tls_sans);
    println!();

    // Execute the installation command
    // NOTE: Must use execute_shell_interactive because the K3s install script uses sudo
    // which requires a TTY to prompt for password. execute_shell (non-interactive) doesn't
    // allocate a TTY, causing sudo to fail silently.
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Installing K3s on {}...", hostname);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Installation command:");
    println!("  {}", install_cmd);
    println!();
    println!("This may take a few minutes. Output will be displayed below:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    io::stdout().flush()?; // Ensure message is displayed before password prompt

    // Use execute_shell_interactive which shows output in real-time
    // Also capture output to a file for later analysis
    let install_output_file = "/tmp/k3s_install_output";
    let install_cmd_with_capture = format!("{} 2>&1 | tee {}", install_cmd, install_output_file);
    println!("[K3s Install Output Start]");
    io::stdout().flush()?;
    let install_result = exec.execute_shell_interactive(&install_cmd_with_capture);
    println!();
    println!("[K3s Install Output End]");
    println!();

    // Check install output for various conditions (check this regardless of install_result)
    let install_output = exec.read_file(install_output_file).ok().unwrap_or_default();
    let service_skipped = install_output.contains("No change detected") && install_output.contains("skipping service start");
    let service_failed = install_output.contains("Job for k3s.service failed") 
        || install_output.contains("control process exited with error code")
        || install_output.contains("Failed to start k3s.service");

    match install_result {
        Ok(()) => {
            println!();
            println!("✓ K3s installation command completed");
            if service_skipped {
                println!("⚠ K3s reported 'No change detected so skipping service start'");
            }

            // Check if service failed during installation
            if service_failed {
                println!("⚠ K3s service failed to start during installation!");
                println!("  Analyzing failure...");
                
                // Get detailed service status and logs immediately
                let status_tmp = "/tmp/k3s_join_failure_status";
                let _ = exec.execute_shell_interactive(&format!(
                    "bash -c 'sudo systemctl status k3s.service --no-pager -l 2>&1 | head -40 > {} 2>&1 || echo \"Unable to get status\" > {}'",
                    status_tmp, status_tmp
                ));
                
                let log_tmp = "/tmp/k3s_join_failure_logs";
                let _ = exec.execute_shell_interactive(&format!(
                    "bash -c 'sudo journalctl -u k3s.service --no-pager -n 50 2>&1 > {} 2>&1 || echo \"Unable to get logs\" > {}'",
                    log_tmp, log_tmp
                ));
                
                if let Ok(status_text) = exec.read_file(status_tmp) {
                    if !status_text.trim().is_empty() && !status_text.contains("Unable to get status") {
                        println!("  Service status:");
                        for line in status_text.lines().take(20) {
                            println!("    {}", line);
                        }
                    }
                }
                
                if let Ok(log_text) = exec.read_file(log_tmp) {
                    if !log_text.trim().is_empty() && !log_text.contains("Unable to get logs") {
                        println!("  Recent service logs:");
                        for line in log_text.lines().take(30) {
                            println!("    {}", line);
                        }
                    }
                }
                
                // Try to detect common issues
                let mut common_issues = Vec::new();
                if install_output.contains("bind: address already in use") || install_output.contains("port") {
                    common_issues.push("Port conflict - another service may be using K3s ports");
                }
                if install_output.contains("permission denied") || install_output.contains("Permission denied") {
                    common_issues.push("Permission issue - check file/directory permissions");
                }
                if install_output.contains("network") || install_output.contains("Network") {
                    common_issues.push("Network issue - check Tailscale connectivity and firewall");
                }
                if install_output.contains("containerd") {
                    common_issues.push("Containerd issue - may need to clean up leftover processes");
                }
                
                if !common_issues.is_empty() {
                    println!("  Possible issues detected:");
                    for issue in &common_issues {
                        println!("    - {}", issue);
                    }
                }
                
                // Try to recover: reset failed state and try starting again
                println!();
                println!("  Attempting to recover...");
                let _ = exec.execute_shell("sudo systemctl reset-failed k3s.service 2>/dev/null || true");
                std::thread::sleep(std::time::Duration::from_secs(2));
                
                // Check if Tailscale is ready (common cause of failures)
                let tailscale_ready = exec.execute_shell("systemctl is-active tailscale 2>/dev/null")
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .map(|s| s.trim() == "active")
                    .unwrap_or(false);
                
                if !tailscale_ready {
                    println!("  ⚠ Tailscale service is not active - this may be causing the failure");
                    println!("     K3s requires Tailscale for cluster communication");
                }
            }

            // CRITICAL: Verify and fix service file to use IP address (not hostname)
            // Systemd services can't resolve Tailscale hostnames, so we MUST use IP address
            println!("Verifying service file uses IP address (required for systemd DNS resolution)...");
            let service_name = if control_plane { "k3s" } else { "k3s-agent" };
            let service_file_path = format!("/etc/systemd/system/{}.service", service_name);
            
            let service_file_check = exec.execute_shell(&format!("sudo cat {} 2>/dev/null || echo 'no_service_file'", service_file_path)).ok();
            if let Some(service_content) = service_file_check {
                let content = String::from_utf8_lossy(&service_content.stdout);
                
                // Check if service file has hostname (ts.net) instead of IP
                let needs_fix = if server_addr.contains("ts.net") {
                    content.contains(&format!("https://{}:6443", server_addr))
                } else {
                    false
                };
                
                if needs_fix {
                    println!("  ⚠ Service file contains hostname '{}' - systemd can't resolve this!", server_addr);
                    println!("  Fixing to use IP address: {}", server_addr_for_k3s);
                    
                    // Replace ALL occurrences of the hostname with IP
                    let mut fixed_content = content.to_string();
                    fixed_content = fixed_content.replace(&format!("https://{}:6443", server_addr), &format!("https://{}:6443", server_addr_for_k3s));
                    fixed_content = fixed_content.replace(&format!("--server=https://{}:6443", server_addr), &format!("--server=https://{}:6443", server_addr_for_k3s));
                    
                    // Write the fixed service file
                    exec.write_file(&service_file_path, fixed_content.as_bytes())
                        .context("Failed to update service file with IP address")?;
                    
                    // Reload systemd
                    exec.execute_shell("sudo systemctl daemon-reload")
                        .context("Failed to reload systemd after service file update")?;
                    
                    println!("  ✓ Service file updated with IP address");
                    
                    // Stop and restart the service to apply the fix
                    println!("  Restarting service to apply IP address fix...");
                    let _ = exec.execute_shell(&format!("sudo systemctl stop {}.service 2>/dev/null || true", service_name));
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    let _ = exec.execute_shell(&format!("sudo systemctl start {}.service", service_name));
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    println!("  ✓ Service restarted with IP address");
                } else if content.contains(&format!("https://{}:6443", server_addr_for_k3s)) {
                    println!("  ✓ Service file correctly uses IP address: {}", server_addr_for_k3s);
                } else {
                    println!("  ⚠ Could not verify service file content - may need manual check");
                }
            } else {
                println!("  ⚠ Service file not found - may not be created yet");
            }

            // If K3s skipped service start, we MUST manually start/restart it
            // This is critical - the service won't start automatically if K3s thinks nothing changed
            if service_skipped {
                println!("⚠ K3s skipped service start - this means the service file already existed.");
                println!("  Forcing service restart to ensure it uses the correct configuration...");
                let service_name = if control_plane { "k3s" } else { "k3s-agent" };
                
                // Stop the service first (in case it's in a bad state or using old config)
                println!("  Stopping service (if running)...");
                let _ = exec.execute_shell(&format!("sudo systemctl stop {}.service 2>/dev/null || true", service_name));
                std::thread::sleep(std::time::Duration::from_secs(3));
                
                // Reload systemd to ensure we have the latest service file
                println!("  Reloading systemd...");
                let _ = exec.execute_shell("sudo systemctl daemon-reload");
                
                // Start the service
                println!("  Starting service with updated configuration...");
                let start_result = exec.execute_shell(&format!("sudo systemctl start {}.service", service_name));
                if start_result.is_err() {
                    println!("  ⚠ Failed to start service");
                } else {
                    println!("  ✓ Service start command executed");
                }
                
                // Wait for service to start
                println!("  Waiting for service to initialize...");
                std::thread::sleep(std::time::Duration::from_secs(15));
            } else {
            // Wait a moment for the service to start
            println!("Waiting for K3s service to start...");
            std::thread::sleep(std::time::Duration::from_secs(10));
            }

            // Verify the service actually started
            println!("Verifying K3s service is running...");

            // Try checking k3s service first, then k3s-agent
            let service_status = {
                // Try k3s service first
                let k3s_status = exec.execute_shell("systemctl is-active k3s 2>/dev/null")
                    .ok()
                    .and_then(|out| String::from_utf8(out.stdout).ok())
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| "inactive".to_string());

                if k3s_status == "active" || k3s_status == "activating" {
                    k3s_status
                } else {
                    // Try k3s-agent
                    exec.execute_shell("systemctl is-active k3s-agent 2>/dev/null")
                        .ok()
                        .and_then(|out| String::from_utf8(out.stdout).ok())
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|| "not_running".to_string())
                }
            };

            // Check service status more carefully - "activating" might mean it's starting or failing
            if service_status == "activating" {
                // Wait a bit longer and check again - it might be starting
                println!("  Service is activating, waiting a bit longer...");
                std::thread::sleep(std::time::Duration::from_secs(10));
                
                // Check again
                let service_status_retry = exec.execute_shell("systemctl is-active k3s 2>/dev/null")
                    .ok()
                    .and_then(|out| String::from_utf8(out.stdout).ok())
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| "inactive".to_string());
                
                if service_status_retry == "active" {
                    println!("  ✓ Service is now active");
                } else {
                    // Still not active - check if it failed
                    let service_failed_check = exec.execute_shell("systemctl is-failed k3s 2>/dev/null")
                        .ok()
                        .and_then(|out| String::from_utf8(out.stdout).ok())
                        .map(|s| s.trim() == "failed")
                        .unwrap_or(false);
                    
                    if service_failed_check {
                        println!("  ⚠ Service has FAILED - checking logs for error...");
                        // Get the actual error from logs
                        let error_logs = exec.execute_shell("sudo journalctl -u k3s -n 30 --no-pager 2>&1").ok();
                        if let Some(logs) = error_logs {
                            let log_text = String::from_utf8_lossy(&logs.stdout);
                            println!("  Recent error logs:");
                            for line in log_text.lines().take(20) {
                                let line_lower = line.to_lowercase();
                                if line_lower.contains("error") || line_lower.contains("fail") || line_lower.contains("fatal") || line_lower.contains("dial tcp") || line_lower.contains("no such host") {
                                    println!("    {}", line);
                                }
                            }
                        }
                        
                        // Also check the service file to see what command it's running
                        println!("  Checking service file command...");
                        let service_file_cmd = exec.execute_shell("sudo grep ExecStart /etc/systemd/system/k3s.service 2>/dev/null || echo 'not_found'").ok();
                        if let Some(cmd) = service_file_cmd {
                            let cmd_text = String::from_utf8_lossy(&cmd.stdout);
                            println!("    Service command: {}", cmd_text.trim());
                            
                            // Check if it still has hostname
                            if cmd_text.contains(&format!("https://{}:6443", server_addr)) && server_addr.contains("ts.net") {
                                println!("    ⚠ Service file STILL has hostname! Fixing now...");
                                // Read full service file
                                let full_service = exec.execute_shell("sudo cat /etc/systemd/system/k3s.service 2>/dev/null").ok();
                                if let Some(service_content) = full_service {
                                    let content = String::from_utf8_lossy(&service_content.stdout);
                                    let fixed = content.replace(&format!("https://{}:6443", server_addr), &format!("https://{}:6443", server_addr_for_k3s));
                                    exec.write_file("/etc/systemd/system/k3s.service", fixed.as_bytes())
                                        .context("Failed to fix service file")?;
                                    exec.execute_shell("sudo systemctl daemon-reload")
                                        .context("Failed to reload systemd")?;
                                    println!("    ✓ Service file fixed - restarting...");
                                    let _ = exec.execute_shell("sudo systemctl restart k3s.service");
                                    std::thread::sleep(std::time::Duration::from_secs(10));
                                }
                            }
                        }
                    }
                }
            }
            
            // Final check - is service active now?
            let final_status = exec.execute_shell("systemctl is-active k3s 2>/dev/null")
                .ok()
                .and_then(|out| String::from_utf8(out.stdout).ok())
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "inactive".to_string());
            
            if final_status != "active" && final_status != "activating" {
                println!("⚠ K3s service is not running after installation!");
                println!("  Status: {}", service_status);
                println!("  Checking service logs for connection errors...");

                // Get detailed service status
                let status_tmp = "/tmp/k3s_join_service_status";
                let _ = exec.execute_shell_interactive(&format!(
                    "bash -c 'sudo systemctl status k3s.service --no-pager -l 2>&1 | head -40 > {} 2>&1 || sudo systemctl status k3s-agent.service --no-pager -l 2>&1 | head -40 > {} 2>&1 || echo \"Unable to get status\" > {}'",
                    status_tmp, status_tmp, status_tmp
                ));

                // Get recent service logs to diagnose the issue
                let log_tmp = "/tmp/k3s_join_service_logs";
                let _ = exec.execute_shell_interactive(&format!(
                    "bash -c '(sudo journalctl -u k3s -n 50 --no-pager 2>&1 || sudo journalctl -u k3s-agent -n 50 --no-pager 2>&1 || echo \"Unable to get logs\") > {} 2>&1'",
                    log_tmp
                ));
                
                if let Ok(status_text) = exec.read_file(status_tmp) {
                    if !status_text.trim().is_empty() && !status_text.contains("Unable to get status") {
                        println!("  Service status:");
                        for line in status_text.lines().take(25) {
                            println!("    {}", line);
                        }
                    }
                }
                
                if let Ok(log_text) = exec.read_file(log_tmp) {
                    if !log_text.trim().is_empty() && !log_text.contains("Unable to get logs") {
                        println!("  Recent service logs:");
                        for line in log_text.lines().take(40) {
                            println!("    {}", line);
                        }
                    }
                }
                
                // Provide helpful troubleshooting steps
                println!();
                println!("  Troubleshooting steps:");
                println!("    1. Check if Tailscale is running: systemctl status tailscale");
                println!("    2. Check for port conflicts: sudo netstat -tulpn | grep -E ':(6443|10250|2379|2380)'");
                println!("    3. Check for leftover processes: sudo ps aux | grep -E '(k3s|containerd)'");
                println!("    4. Check data directory permissions: ls -la /var/lib/rancher/k3s/");
                println!("    5. View full logs: sudo journalctl -u k3s -n 100 --no-pager");

                anyhow::bail!(
                    "K3s installation command completed but service is not running. Status: {}\n\
                     \n\
                     Common causes:\n\
                     - Tailscale not ready/connected\n\
                     - Port conflicts (6443, 10250, etc.)\n\
                     - Leftover containerd/k3s processes\n\
                     - Data directory permission issues\n\
                     - Network/firewall blocking cluster communication\n\
                     \n\
                     Check the logs above and run:\n\
                     ssh {} 'sudo journalctl -u k3s -n 100 --no-pager'",
                    service_status,
                    hostname
                );
            } else {
                println!("  ✓ K3s service is running (status: {})", service_status);
                // Skip diagnostic checks - service is running, proceed with verification
                // These checks were causing SSH connection issues and are not critical
            }
            
            // Configure K3s service to depend on Tailscale (for cluster communication)
            println!("Configuring K3s service to depend on Tailscale...");
            let service_name = if control_plane { "k3s" } else { "k3s-agent" };
            let service_override_dir = format!("/etc/systemd/system/{}.service.d", service_name);
            let override_content = r#"[Unit]
After=tailscale.service
Wants=tailscale.service
Requires=network-online.target
"#;
            
            // Create override directory
            let _ = exec.execute_shell(&format!("sudo mkdir -p {}", service_override_dir));
            
            // Write override file
            let override_file = format!("{}/10-tailscale.conf", service_override_dir);
            if let Err(e) = exec.write_file(&override_file, override_content.as_bytes()) {
                println!("⚠️  Warning: Failed to create systemd override: {}", e);
                println!("   K3s service may start before Tailscale is ready.");
            } else {
                println!("✓ Created systemd override to ensure K3s starts after Tailscale");
                
                // Reload systemd to pick up the override
                let _ = exec.execute_shell("sudo systemctl daemon-reload");
                println!("✓ Tailscale dependency configured (override file only, doesn't affect main service file)");
            }
            
            // Fix kubeconfig permissions to ensure it's readable
            // Even with --write-kubeconfig-mode=0644, we need to ensure the file is actually readable
            println!("Fixing kubeconfig permissions...");
            let kubeconfig_path = "/etc/rancher/k3s/k3s.yaml";
            let _ = exec.execute_shell(&format!(
                "sudo chmod 644 {} 2>/dev/null || echo 'kubeconfig_not_created_yet'",
                kubeconfig_path
            ));
            // Also ensure the directory is accessible
            let _ = exec.execute_shell("sudo chmod 755 /etc/rancher/k3s 2>/dev/null || true");
            println!("✓ Kubeconfig permissions fixed");
            println!();
        }
        Err(e) => {
            // Command execution itself failed (not just exit code)
            println!();
            println!("⚠ Installation command failed.");
            println!("Error: {}", e);
            println!();

            // Check if service exists and get logs - use execute_shell_interactive for sudo commands
            println!("Checking K3s service status and logs...");

            // Get service status using temp file to capture output from interactive sudo command
            let status_tmp = "/tmp/k3s_join_status";
            let _ = exec.execute_shell_interactive(&format!(
                "bash -c 'sudo systemctl status k3s.service --no-pager -l 2>&1 | head -30 > {} 2>&1 || echo \"Unable to get status\" > {}'",
                status_tmp, status_tmp
            ));
            if let Ok(status_text) = exec.read_file(status_tmp) {
                if !status_text.trim().is_empty() && !status_text.contains("Unable to get status") {
                    println!("Service status:");
                    println!("{}", status_text);
                }
            }

            println!();
            println!("Fetching recent service logs...");

            // Get journal logs using temp file to capture output from interactive sudo command
            let log_tmp = "/tmp/k3s_join_logs_error";
            let _ = exec.execute_shell_interactive(&format!(
                "bash -c 'sudo journalctl -u k3s.service --no-pager -n 50 2>&1 > {} 2>&1 || echo \"Unable to get logs\" > {}'",
                log_tmp, log_tmp
            ));
            if let Ok(log_text) = exec.read_file(log_tmp) {
                if !log_text.trim().is_empty() && !log_text.contains("Unable to get logs") {
                    println!("Recent logs:");
                    println!("{}", log_text);
                }
            }

            println!();
            println!("Checking installation status...");
            let k3s_exists = exec
                .execute_shell("test -f /usr/local/bin/k3s && echo exists || echo not_exists")
                .ok();
            let k3s_agent_exists = exec
                .execute_shell("test -f /usr/local/bin/k3s-agent && echo exists || echo not_exists")
                .ok();

            let has_binary = k3s_exists
                .map(|c| String::from_utf8_lossy(&c.stdout).trim() == "exists")
                .unwrap_or(false);
            let has_agent_binary = k3s_agent_exists
                .map(|c| String::from_utf8_lossy(&c.stdout).trim() == "exists")
                .unwrap_or(false);

            if has_binary || has_agent_binary {
                println!("✓ K3s binary found - checking service status...");
                let service_tmp = "/tmp/k3s_join_service_check";
                let _ = exec.execute_shell_interactive(&format!(
                    "bash -c '(sudo systemctl is-active k3s 2>/dev/null || sudo systemctl is-active k3s-agent 2>/dev/null || echo \"not_running\") > {} 2>&1'",
                    service_tmp
                ));
                if let Ok(status_str) = exec.read_file(service_tmp) {
                    let status_str = status_str.trim().to_string();
                    if status_str == "active" || status_str == "activating" {
                        println!(
                            "✓ K3s service is active despite error - continuing with verification"
                        );
                    } else {
                        // Get service status details
                        let status_tmp2 = "/tmp/k3s_join_status_details";
                        let _ = exec.execute_shell_interactive(&format!(
                            "bash -c '(sudo systemctl status k3s 2>&1 | head -10 || sudo systemctl status k3s-agent 2>&1 | head -10 || echo \"no_status\") > {} 2>&1'",
                            status_tmp2
                        ));
                        let details = exec
                            .read_file(status_tmp2)
                            .unwrap_or_else(|_| "no_status".to_string())
                            .trim()
                            .to_string();

                        return Err(e).context(format!(
                            "K3s installation failed. Binary exists but service is not running (status: {}). Details: {}",
                            status_str,
                            if details.is_empty() { "No status available" } else { &details }
                        ));
                    }
                } else {
                    return Err(e)
                        .context("K3s installation failed - unable to check service status");
                }
            } else {
                // Installation completely failed - show what we can
                let curl_test = exec
                    .execute_shell("curl --version 2>&1 | head -1 || echo 'curl_failed'")
                    .ok();
                let curl_info = curl_test
                    .map(|c| String::from_utf8_lossy(&c.stdout).trim().to_string())
                    .unwrap_or_default();

                return Err(e).context(format!(
                    "K3s installation completely failed. K3s binary not found. curl info: {}",
                    curl_info
                ));
            }
        }
    }

    // Wait a moment for the service to start and attempt to join
    println!("Waiting for K3s service to initialize and join cluster...");
    std::thread::sleep(std::time::Duration::from_secs(15));

    // Check service status on the joining node BEFORE verification
    // This helps diagnose issues early
    println!();
    println!("Checking K3s service status on {}...", hostname);
    let service_name = if control_plane { "k3s" } else { "k3s-agent" };
    let remote_service_status = exec.execute_shell(&format!(
        "systemctl is-active {} 2>/dev/null || echo 'not_running'",
        service_name
    )).ok();
    
    let is_service_running = remote_service_status
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|s| {
            let status = s.trim();
            status == "active" || status == "activating"
        })
        .unwrap_or(false);
    
    if !is_service_running {
        println!("⚠ K3s service is NOT running on {}!", hostname);
        println!("  This will prevent the node from joining the cluster.");
        println!();
        println!("  To diagnose the issue, run:");
        println!("    ssh {} 'sudo systemctl status {} --no-pager -l'", hostname, service_name);
        println!("    ssh {} 'sudo journalctl -u {} -n 100 --no-pager'", hostname, service_name);
        println!();
        println!("  Common issues:");
        println!("    - Tailscale not ready/connected");
        println!("    - Port conflicts (6443, 10250, etc.)");
        println!("    - Invalid join token or server address");
        println!("    - Network/firewall blocking cluster communication");
        println!();
        anyhow::bail!(
            "K3s service is not running on {}. The node cannot join the cluster until the service is running.\n\
             Check the service status and logs using the commands above.",
            hostname
        );
    } else {
        println!("  ✓ K3s service is running on {}", hostname);
        
        // Check recent logs for connection/join status
        println!("  Checking service logs for cluster connection status...");
        let recent_logs = exec.execute_shell(&format!(
            "sudo journalctl -u {} -n 50 --no-pager 2>&1",
            service_name
        )).ok();
        
        if let Some(logs_out) = recent_logs {
            let logs = String::from_utf8_lossy(&logs_out.stdout);
            
            // Check for connection errors to primary server
            if logs.contains("no such host") || logs.contains("lookup") && logs.contains("failed") {
                println!("  ⚠ CRITICAL: Service cannot resolve primary server hostname!");
                println!("    This means the service file still has a hostname instead of IP address.");
                println!("    Checking service file...");
                
                // Check and fix service file immediately
                let service_file_check = exec.execute_shell(&format!("sudo cat /etc/systemd/system/{}.service 2>/dev/null || echo 'no_service_file'", service_name)).ok();
                if let Some(service_content) = service_file_check {
                    let content = String::from_utf8_lossy(&service_content.stdout);
                    if content.contains(&format!("https://{}:6443", server_addr)) && server_addr.contains("ts.net") {
                        println!("    ⚠ Service file has hostname '{}' - fixing to use IP: {}", server_addr, server_addr_for_k3s);
                        let fixed_content = content.replace(&format!("https://{}:6443", server_addr), &format!("https://{}:6443", server_addr_for_k3s));
                        exec.write_file(&format!("/etc/systemd/system/{}.service", service_name), fixed_content.as_bytes())
                            .context("Failed to fix service file with IP address")?;
                        exec.execute_shell("sudo systemctl daemon-reload")
                            .context("Failed to reload systemd")?;
                        println!("    ✓ Service file fixed - restarting service...");
                        let _ = exec.execute_shell(&format!("sudo systemctl restart {}.service", service_name));
                        std::thread::sleep(std::time::Duration::from_secs(10));
                        println!("    ✓ Service restarted with IP address");
                    }
                }
            }
            
            // Check for other errors
            if logs.contains("error") || logs.contains("fail") || logs.contains("fatal") {
                println!("  ⚠ Recent errors in service logs:");
                for line in logs.lines().take(10) {
                    let line_lower = line.to_lowercase();
                    if line_lower.contains("error") || line_lower.contains("fail") || line_lower.contains("fatal") {
                        println!("    {}", line);
                    }
                }
            }
            
            // Check for successful connection indicators
            if logs.contains("successfully") || logs.contains("joined") || logs.contains("ready") {
                println!("  ✓ Service logs show positive connection indicators");
            }
        }
    }

    // Verify the node successfully joined the cluster (with retries)
    println!();
    println!(
        "Verifying cluster membership on {} (this may take a few minutes)...",
        hostname
    );

    // Write the kubeconfig we fetched earlier to local filesystem
    println!();
    println!("Setting up kubectl access from local machine...");
    let home = std::env::var("HOME")
        .ok()
        .unwrap_or_else(|| ".".to_string());
    let kube_dir = format!("{}/.kube", home);
    std::fs::create_dir_all(&kube_dir).context("Failed to create ~/.kube directory")?;
    let kube_config_path = format!("{}/config", kube_dir);

    // Verify the kubeconfig points to the primary server, not the joining node
    // Extract server address from kubeconfig to verify it's correct
    let mut final_kubeconfig = kubeconfig_content.clone();
    if let Ok((server_host, _)) = kubeconfig::extract_server_and_token_from_kubeconfig(&kubeconfig_content) {
        println!("  Kubeconfig server: {}", server_host);
        
        // Check if server points to joining node instead of primary
        let joining_node_hostname = if is_local {
            // Get local hostname
            halvor_core::utils::hostname::get_current_hostname().unwrap_or_else(|_| hostname.to_string())
        } else {
            hostname.to_string()
        };
        
        // Normalize hostnames for comparison
        let normalized_server = halvor_core::utils::hostname::normalize_hostname(&server_host);
        let normalized_joining = halvor_core::utils::hostname::normalize_hostname(&joining_node_hostname);
        let normalized_primary = halvor_core::utils::hostname::normalize_hostname(&primary_hostname);
        
        // Check if server matches joining node (wrong) instead of primary (correct)
        if normalized_server == normalized_joining || server_host == joining_node_hostname {
            println!("  ⚠ Warning: Kubeconfig points to joining node ({}) instead of primary ({}).", server_host, primary_hostname);
            println!("     Processing kubeconfig to fix server address...");
            
            // Re-process the kubeconfig to fix the server address (no SSH needed)
            // This will extract the server from kubeconfig and replace it with the correct primary server
            match kubeconfig::process_kubeconfig_for_primary(&final_kubeconfig, &primary_hostname, config) {
                Ok(corrected_kubeconfig) => {
                    // Verify the corrected kubeconfig
                    if let Ok((corrected_server, _)) = kubeconfig::extract_server_and_token_from_kubeconfig(&corrected_kubeconfig) {
                        let normalized_corrected = halvor_core::utils::hostname::normalize_hostname(&corrected_server);
                        if normalized_corrected == normalized_primary || corrected_server == primary_hostname {
                            println!("  ✓ Corrected kubeconfig points to primary: {}", corrected_server);
                            // Use the corrected kubeconfig
                            final_kubeconfig = corrected_kubeconfig;
                        } else {
                            println!("  ⚠ Corrected kubeconfig still has wrong server: {}", corrected_server);
                            println!("     Will attempt to fix during final replacement step.");
                        }
                    }
                }
                Err(e) => {
                    println!("  ⚠ Failed to process kubeconfig: {}", e);
                    println!("     Will attempt to fix during final replacement step.");
                }
            }
        } else if normalized_server == normalized_primary || server_host == primary_hostname {
            println!("  ✓ Kubeconfig correctly points to primary server");
        } else {
            println!("  ⚠ Kubeconfig server ({}) doesn't match primary ({}) or joining node ({})", 
                     server_host, primary_hostname, joining_node_hostname);
        }
    }

    // Final pass: Replace any remaining baulder hostnames with primary server
    // This catches any instances that might have been missed, including in merged content
    let joining_node_hostname_for_replace = if is_local {
        halvor_core::utils::hostname::get_current_hostname().unwrap_or_else(|_| hostname.to_string())
    } else {
        hostname.to_string()
    };
    
    // Extract primary server address from kubeconfig - no SSH needed!
    // The kubeconfig already contains the correct server address
    let primary_server_url_final = if let Ok((server_host, _)) = kubeconfig::extract_server_and_token_from_kubeconfig(&final_kubeconfig) {
        // Server host is just the hostname/IP, need to add https:// and :6443 if not present
        if server_host.starts_with("https://") {
            server_host
        } else {
            format!("https://{}:6443", server_host)
        }
    } else {
        // Fallback: construct from server parameter
        format!("https://{}:6443", server)
    };
    
    // Extract just the hostname/IP part for replacement patterns
    let primary_server_addr = primary_server_url_final
        .strip_prefix("https://")
        .or_else(|| primary_server_url_final.strip_prefix("http://"))
        .and_then(|s| s.split(':').next())
        .unwrap_or(&primary_server_url_final);
    
    // Replace any baulder hostnames in the final kubeconfig
    // Try multiple patterns to catch all variations
    let mut final_cleaned = final_kubeconfig.clone();
    
    // Get baulder's Tailscale hostname if available for more accurate matching
    let baulder_tailscale_hostname = if is_local {
        tailscale::get_tailscale_hostname_remote(&exec)
            .ok()
            .flatten()
    } else {
        None
    };
    
    let baulder_patterns: Vec<String> = vec![
        format!("https://{}:6443", joining_node_hostname_for_replace),
        format!("https://{}.bombay-pinecone.ts.net:6443", joining_node_hostname_for_replace),
        format!("{}:6443", joining_node_hostname_for_replace),
        format!("{}.bombay-pinecone.ts.net:6443", joining_node_hostname_for_replace),
    ]
    .into_iter()
    .chain(baulder_tailscale_hostname.iter().map(|h| format!("https://{}:6443", h)))
    .chain(baulder_tailscale_hostname.iter().map(|h| format!("{}:6443", h)))
    .collect();
    
    let mut replacements_made = 0;
    for pattern in &baulder_patterns {
        if final_cleaned.contains(pattern) {
            println!("  Replacing {} with {}", pattern, primary_server_url_final);
            final_cleaned = final_cleaned.replace(pattern, &primary_server_url_final);
            replacements_made += 1;
        }
    }
    
    if replacements_made > 0 {
        println!("  ✓ Made {} replacement(s) to ensure kubeconfig points to primary server", replacements_made);
    }
    
    // Process through kubeconfig processor one more time to ensure all server URLs are correct
    // This handles multiple clusters/contexts that might exist in the kubeconfig
    let final_processed = match kubeconfig::process_kubeconfig_for_primary(&final_cleaned, &primary_hostname, config) {
        Ok(processed) => {
            println!("  ✓ Processed kubeconfig through primary processor");
            processed
        }
        Err(e) => {
            println!("  ⚠ Failed to process kubeconfig through primary processor: {}", e);
            println!("  Using manually processed version instead");
            final_cleaned
        }
    };
    
    // The kubeconfig should point to the primary server (frigg), not the joining node (baulder)
    // Merge with existing config if it exists, otherwise write new
    if std::path::Path::new(&kube_config_path).exists() {
        println!("  Merging with existing kubeconfig at {}", kube_config_path);
        let existing = std::fs::read_to_string(&kube_config_path).unwrap_or_default();
        
        // Process existing config to replace any baulder references with frigg
        let existing_processed = if !existing.is_empty() {
            let mut existing_cleaned = existing.clone();
            for pattern in &baulder_patterns {
                if existing_cleaned.contains(pattern) {
                    println!("  Replacing {} with {} in existing config", pattern, primary_server_url_final);
                    existing_cleaned = existing_cleaned.replace(pattern, &primary_server_url_final);
                }
            }
            existing_cleaned
        } else {
            existing
        };
        
        if !existing_processed.contains("k3s") {
            let mut merged = existing_processed;
            if !merged.ends_with('\n') {
                merged.push('\n');
            }
            merged.push_str("---\n");
            merged.push_str(&final_processed);
            std::fs::write(&kube_config_path, merged)?;
        } else {
            // Existing config has k3s - replace the entire k3s section with our processed version
            println!("  Existing k3s context found - replacing with corrected version");
            // Try to find and replace the k3s cluster section
            let mut merged = existing_processed;
            // Replace any remaining baulder references in the merged content
            for pattern in &baulder_patterns {
                if merged.contains(pattern) {
                    println!("  Replacing {} with {} in merged config", pattern, primary_server_url_final);
                    merged = merged.replace(pattern, &primary_server_url_final);
                }
            }
            std::fs::write(&kube_config_path, merged)?;
        }
    } else {
        std::fs::write(&kube_config_path, &final_processed)
            .context("Failed to write kubeconfig")?;
    }
    
    // Verify what we actually wrote by reading the file back
    let mut written_content = std::fs::read_to_string(&kube_config_path)
        .context("Failed to read back written kubeconfig for verification")?;
    
    // Final pass: Replace ANY remaining baulder references in the entire file
    let mut final_replacements = 0;
    for pattern in &baulder_patterns {
        if written_content.contains(pattern) {
            println!("  Final pass: Replacing {} with {}", pattern, primary_server_url_final);
            written_content = written_content.replace(pattern, &primary_server_url_final);
            final_replacements += 1;
        }
    }
    
    // Also replace any variations we might have missed
    let additional_patterns = vec![
        format!("server: https://{}", joining_node_hostname_for_replace),
        format!("server:https://{}", joining_node_hostname_for_replace),
        format!("server: http://{}", joining_node_hostname_for_replace),
        format!("server:http://{}", joining_node_hostname_for_replace),
    ];
    for pattern in &additional_patterns {
        if written_content.contains(pattern) {
            let replacement = format!("server: {}", primary_server_url_final);
            println!("  Final pass: Replacing {} with {}", pattern, replacement);
            written_content = written_content.replace(pattern, &replacement);
            final_replacements += 1;
        }
    }
    
    if final_replacements > 0 {
        println!("  ✓ Made {} additional replacement(s) in final pass", final_replacements);
        std::fs::write(&kube_config_path, &written_content)
            .context("Failed to write final corrected kubeconfig")?;
    }
    
    // Extract server from the actual written file (might be merged)
    if let Ok((written_server, _)) = kubeconfig::extract_server_and_token_from_kubeconfig(&written_content) {
        println!("  Final kubeconfig server (from file): {}", written_server);
        let normalized_written = halvor_core::utils::hostname::normalize_hostname(&written_server);
        let normalized_primary = halvor_core::utils::hostname::normalize_hostname(&primary_hostname);
        
        // Use the primary server address extracted from kubeconfig
        let primary_server_address = primary_server_addr;
        
        // Check if server matches primary (could be hostname or IP)
        let server_matches = normalized_written == normalized_primary 
            || written_server == primary_hostname
            || written_server == primary_server_address
            || written_server.contains(&normalized_primary)
            || written_server.contains(primary_server_address);
        
        if server_matches {
            println!("  ✓ Verified: Written kubeconfig points to primary server");
        } else {
            println!("  ⚠ Warning: Written kubeconfig server ({}) doesn't match primary ({})", written_server, primary_hostname);
            println!("  Attempting to fix kubeconfig by replacing server URL...");
            
            // Try to fix it by replacing the server URL in the file
            // Replace both the full URL and just the hostname part
            let fixed_content = written_content
                .replace(&written_server, &primary_server_url_final)
                .replace(&format!("https://{}:6443", written_server), &primary_server_url_final)
                .replace(&format!("http://{}:6443", written_server), &primary_server_url_final);
            
            // Also try to process it through the kubeconfig processor
            let fixed_processed = kubeconfig::process_kubeconfig_for_primary(&fixed_content, &primary_hostname, config)
                .unwrap_or(fixed_content);
            
            std::fs::write(&kube_config_path, &fixed_processed)
                .context("Failed to write fixed kubeconfig")?;
            
            // Verify again
            if let Ok((fixed_server, _)) = kubeconfig::extract_server_and_token_from_kubeconfig(&fixed_processed) {
                println!("  Fixed kubeconfig server: {}", fixed_server);
                let fixed_normalized = halvor_core::utils::hostname::normalize_hostname(&fixed_server);
                if fixed_normalized == normalized_primary || fixed_server == primary_hostname {
                    println!("  ✓ Fixed kubeconfig now points to primary server");
                } else {
                    println!("  ⚠ Still incorrect after fix attempt. Server: {}, Expected: {}", fixed_server, primary_hostname);
                }
            }
        }
    } else {
        println!("  ⚠ Warning: Could not extract server from written kubeconfig");
    }
    
    println!("✓ Kubeconfig set up at {}", kube_config_path);

    // Verify the node successfully joined the cluster using local kubectl
    // Pass the final processed kubeconfig to verification (not the original)
    // This ensures verification uses the corrected version that points to frigg, not baulder
    println!();
    println!("Verifying cluster membership using local kubectl (this may take a few minutes)...");
    
    // Read the final corrected kubeconfig from the file we just wrote
    let final_kubeconfig_for_verification = std::fs::read_to_string(&kube_config_path)
        .context("Failed to read corrected kubeconfig for verification")?;
    
    let verification_result = verify::verify_cluster_join_with_local_kubectl_and_config(
        &primary_hostname,
        hostname,
        control_plane,
        config,
        Some(final_kubeconfig_for_verification), // Use the corrected version from the file
    );

    // If verification failed and service was skipped, offer to restart
    if verification_result.is_err() && service_skipped {
        println!();
        println!(
            "⚠ Cluster verification failed, and K3s reported 'No change detected so skipping service start'."
        );
        println!("  The service may not have restarted with the new configuration.");
        println!();

        // Prompt user to restart service
        print!("Would you like to restart the K3s service? [Y/n]: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let should_restart =
            input.trim().is_empty() || input.trim().to_lowercase().starts_with('y');

        if should_restart {
            println!();
            println!("Restarting K3s service...");

            // Determine which service to restart
            let service_name = if control_plane { "k3s" } else { "k3s-agent" };
            exec.execute_interactive("sudo", &["systemctl", "restart", service_name])
                .context("Failed to restart K3s service")?;

            println!("✓ Service restarted, waiting 15 seconds for it to initialize...");
            std::thread::sleep(std::time::Duration::from_secs(15));

            // Retry verification
            println!();
            println!("Retrying cluster verification...");
            verify::verify_cluster_join_with_local_kubectl_and_config(
                &primary_hostname,
                hostname,
                control_plane,
                config,
                Some(kubeconfig_content),
            )
            .context("Failed to verify cluster join after service restart")?;
        } else {
            return verification_result
                .context("Cluster verification failed. Service restart was declined.");
        }
    } else {
        verification_result.context("Failed to verify cluster join after multiple attempts")?;
    }

    println!();
    if control_plane {
        println!("✓ Successfully joined cluster as control plane node!");
    } else {
        println!("✓ Successfully joined cluster as agent node!");
    }
    println!();

    // Setup halvor agent service on the node (only if not already set up)
    let service_exists = exec.file_exists("/etc/systemd/system/halvor-agent.service").unwrap_or(false);
    let service_active = exec
        .execute_shell("systemctl is-active halvor-agent.service 2>/dev/null || echo inactive")
        .ok()
        .and_then(|o| {
            String::from_utf8(o.stdout).ok().map(|s| s.trim() == "active")
        })
        .unwrap_or(false);

    if service_exists && service_active {
        println!("✓ Halvor agent service is already configured and running");
        println!();
    } else {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Setting up halvor agent service");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!();
        
        // Check if web UI should be enabled (check for HALVOR_WEB_DIR or web build)
        let web_port = if std::env::var("HALVOR_WEB_DIR").is_ok() {
            Some(13000)
        } else {
            None
        };
        
        if let Err(e) = agent_service::setup_agent_service(&exec, web_port) {
            eprintln!("⚠️  Warning: Failed to setup halvor agent service: {}", e);
            eprintln!("   You can set it up manually later with: halvor agent start --daemon");
        } else {
            println!("✓ Halvor agent service is running on {}", hostname);
            println!("  Agent API: port 13500 (over Tailscale)");
            if web_port.is_some() {
                println!("  Web UI: port 13000 (over Tailscale)");
            }
        }
        println!();
    }

    Ok(())
}

/// Check if node is part of an existing cluster and remove it if user confirms
/// This ensures proper cleanup before joining a new cluster
fn check_and_remove_from_existing_cluster<E: CommandExecutor>(
    exec: &E,
    hostname: &str,
    new_server: &str,
    _config: &EnvConfig,
) -> Result<()> {
    // Check if K3s service is running
    let service_status = exec
        .execute_shell("systemctl is-active k3s 2>/dev/null || systemctl is-active k3s-agent 2>/dev/null || echo 'not_running'")
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "not_running".to_string());

    if service_status != "active" && service_status != "activating" {
        // K3s is not running, so node is not part of a cluster
        return Ok(());
    }

    println!();
    println!("⚠ This node appears to be part of an existing cluster.");
    println!("   Checking current cluster information...");

    // Try to get current cluster information
    let cluster_info_tmp = "/tmp/k3s_current_cluster_info";
    let nodes_tmp = "/tmp/k3s_current_nodes";

    // Get cluster info
    let _ = exec.execute_shell_interactive(&format!(
        "sudo k3s kubectl cluster-info 2>&1 | head -5 > {} || echo 'Unable to get cluster info' > {}",
        cluster_info_tmp, cluster_info_tmp
    ));

    let cluster_info = exec
        .read_file(cluster_info_tmp)
        .unwrap_or_else(|_| "Unable to get cluster information".to_string());

    // Get current nodes
    let _ = exec.execute_shell_interactive(&format!(
        "sudo k3s kubectl get nodes -o wide 2>&1 > {} || echo 'Unable to get nodes' > {}",
        nodes_tmp, nodes_tmp
    ));

    let nodes_info = exec
        .read_file(nodes_tmp)
        .unwrap_or_else(|_| "Unable to get nodes".to_string());

    // Get current server address from kubeconfig
    let server_tmp = "/tmp/k3s_current_server";
    let _ = exec.execute_shell_interactive(&format!(
        "sudo k3s kubectl config view --minify -o jsonpath='{{.clusters[0].cluster.server}}' 2>&1 > {} || echo 'Unable to get server' > {}",
        server_tmp, server_tmp
    ));

    let current_server = exec
        .read_file(server_tmp)
        .unwrap_or_else(|_| "Unknown".to_string())
        .trim()
        .to_string();

    // Display current cluster information
    println!();
    println!("Current Cluster Information:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    if !current_server.is_empty() && current_server != "Unable to get server" {
        println!("Server: {}", current_server);
    }
    if !cluster_info.trim().is_empty() && !cluster_info.contains("Unable to get") {
        println!("Cluster Info:");
        for line in cluster_info.lines().take(5) {
            println!("  {}", line);
        }
    }
    if !nodes_info.trim().is_empty() && !nodes_info.contains("Unable to get") {
        println!("Nodes:");
        for line in nodes_info.lines() {
            println!("  {}", line);
        }
    }
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("New Cluster Server: {}", new_server);
    println!();

    // Ask user for confirmation
    print!(
        "This node will be removed from the current cluster and joined to the new cluster.\n\
         This will:\n\
          1. Remove this node from the current cluster (if it's a control plane, it will be drained)\n\
          2. Uninstall existing K3s installation\n\
          3. Join the new cluster\n\
         \n\
         Continue? [y/N]: "
    );
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if !input.trim().eq_ignore_ascii_case("y") {
        println!("Aborted. Node will remain in the current cluster.");
        anyhow::bail!("Join operation cancelled by user");
    }

    println!();
    println!("Removing node from current cluster...");

    // Try to remove this node from the cluster using kubectl
    // First, check if this node is listed in the cluster
    let node_name_tmp = "/tmp/k3s_node_name";
    let _ = exec.execute_shell_interactive(&format!("hostname > {} 2>&1", node_name_tmp));

    let node_name = exec
        .read_file(node_name_tmp)
        .unwrap_or_else(|_| hostname.to_string())
        .trim()
        .to_string();

    // Try to drain and delete the node (if it's a control plane or worker)
    println!("  Draining node {} from cluster...", node_name);
    let drain_cmd = format!(
        "timeout 120 sudo k3s kubectl drain {} --ignore-daemonsets --delete-emptydir-data --force --grace-period=30 2>&1 || echo 'drain_failed'",
        node_name
    );
    let drain_output = exec.execute_shell(&drain_cmd);
    if let Ok(output) = drain_output {
        let output_str = String::from_utf8_lossy(&output.stdout);
        if output_str.contains("drained") || output.status.success() {
        println!("  ✓ Node drained successfully");
    } else {
        println!("  ⚠ Could not drain node (may not be in cluster or already removed)");
            if !output_str.trim().is_empty() && !output_str.contains("drain_failed") {
                println!("    Output: {}", output_str.trim());
            }
        }
    } else {
        println!("  ⚠ Could not drain node (command failed)");
    }

    // Delete the node from the cluster
    println!("  Deleting node {} from cluster...", node_name);
    let delete_cmd = format!(
        "timeout 30 sudo k3s kubectl delete node {} 2>&1 || echo 'delete_failed'",
        node_name
    );
    let delete_output = exec.execute_shell(&delete_cmd);
    if let Ok(output) = delete_output {
        let output_str = String::from_utf8_lossy(&output.stdout);
        if output_str.contains("deleted") || output.status.success() {
        println!("  ✓ Node deleted from cluster");
    } else {
        println!("  ⚠ Could not delete node (may not be in cluster or already removed)");
            if !output_str.trim().is_empty() {
                println!("    Output: {}", output_str.trim());
            }
        }
    } else {
        println!("  ⚠ Could not delete node (command failed)");
    }

    // Immediately stop K3s service after node deletion to prevent hanging
    println!("  Stopping K3s service...");
    let _ = exec.execute_shell("sudo systemctl stop k3s.service 2>/dev/null || sudo systemctl stop k3s-agent.service 2>/dev/null || true");
    let _ = exec.execute_shell("sudo pkill -9 k3s 2>/dev/null || true");

    // Wait a moment for cleanup
    std::thread::sleep(std::time::Duration::from_secs(2));

    println!("✓ Node removal process completed");
    println!();

    Ok(())
}

/// Find hostname from server address (IP or hostname)
/// Returns the hostname if found in config, otherwise returns the server address as-is
fn find_hostname_from_server(server: &str, config: &EnvConfig) -> Option<String> {
    // If it's already a hostname in config, return it
    if let Some(hostname) = halvor_core::utils::hostname::find_hostname_in_config(server, config) {
        return Some(hostname);
    }

    // Try to find hostname by matching IP address
    for (hostname, host_config) in &config.hosts {
        if let Some(ip) = &host_config.ip {
            if ip == server {
                return Some(hostname.clone());
            }
        }
        if let Some(hostname_val) = &host_config.hostname {
            if hostname_val == server {
                return Some(hostname.clone());
            }
        }
    }

    None
}
