//! K3s cluster initialization

use crate::config::EnvConfig;
use crate::services::k3s::utils::generate_cluster_token;
use crate::services::k3s::{agent_service, cleanup, tools};
use crate::services::tailscale;
use crate::utils::exec::{CommandExecutor, Executor};
use anyhow::{Context, Result};
use std::io::{self, Write};

/// Check if an existing K3s cluster is running and return its token if found
fn check_existing_cluster<E: CommandExecutor>(exec: &E) -> Result<Option<String>> {
    // Check if K3s service is running
    let k3s_running = exec
        .execute_shell("systemctl is-active k3s 2>/dev/null || echo inactive")
        .ok()
        .and_then(|o| {
            String::from_utf8(o.stdout).ok().map(|s| s.trim() == "active")
        })
        .unwrap_or(false);

    if !k3s_running {
        return Ok(None);
    }

    // Check if this is a server node (has node-token file)
    // Use sudo to check since the file is owned by root
    let has_node_token = exec
        .execute_shell("sudo test -f /var/lib/rancher/k3s/server/node-token 2>/dev/null && echo exists || echo not_exists")
        .ok()
        .and_then(|o| {
            String::from_utf8(o.stdout).ok().map(|s| s.trim() == "exists")
        })
        .unwrap_or(false);

    if !has_node_token {
        // K3s is running but this might be an agent node, not a server
        return Ok(None);
    }

    // Try to read the current token (write to temp file first, then read it)
    let token_tmp = "/tmp/k3s_cluster_token_check";
    let token_result = exec.execute_shell_interactive(&format!(
        "sudo cat /var/lib/rancher/k3s/server/node-token > {} 2>/dev/null || echo '' > {}",
        token_tmp, token_tmp
    ));

    if token_result.is_ok() {
        if let Ok(token_content) = exec.read_file(token_tmp) {
            let trimmed = token_content.trim().to_string();
            if !trimmed.is_empty() && trimmed != "''" {
                // Clean up temp file
                let _ = exec.execute_shell(&format!("rm -f {}", token_tmp));
                return Ok(Some(trimmed));
            }
        }
    }

    // Clean up temp file
    let _ = exec.execute_shell(&format!("rm -f {}", token_tmp));

    // If we can't read the token but cluster exists, still warn
    Ok(Some("<token unavailable>".to_string()))
}

/// Initialize the first control plane node with embedded etcd
pub fn init_control_plane(
    hostname: &str,
    token: Option<&str>,
    yes: bool,
    config: &EnvConfig,
) -> Result<()> {
    let exec = Executor::new(hostname, config)?;
    let is_local = exec.is_local();

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Initialize K3s HA Cluster - First Control Plane Node");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    if is_local {
        println!("Target: localhost ({})", hostname);
    } else {
        println!("Target: {} (remote)", hostname);
    }
    println!();

    // Check for existing K3s cluster BEFORE generating token or prompting
    println!("Checking for existing K3s cluster...");
    let existing_cluster = check_existing_cluster(&exec)?;
    let had_existing_cluster = existing_cluster.is_some();
    
    if let Some(ref existing_token) = existing_cluster {
        println!();
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("⚠️  WARNING: A K3s cluster already exists on this node!");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!();
        println!("Current cluster token:");
        println!("  {}", existing_token);
        println!();
        println!("⚠️  Initializing a new cluster will:");
        println!("   - Remove the existing K3s installation");
        println!("   - Delete all cluster data and workloads");
        println!("   - Require all nodes to rejoin with the new token");
        println!();
        
        if yes {
            println!("⚠️  --yes flag is set, proceeding with overwrite...");
            println!();
        } else {
            print!("Do you want to overwrite the existing cluster? [y/N]: ");
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            if !input.trim().eq_ignore_ascii_case("y") {
                println!("Aborted. Existing cluster will not be modified.");
                println!();
                // Get Tailscale info for join command
                let tailscale_ip = tailscale::get_tailscale_ip_with_fallback(&exec, hostname, config)
                    .unwrap_or_else(|_| hostname.to_string());
                let tailscale_hostname = tailscale::get_tailscale_hostname_remote(&exec)
                    .ok()
                    .flatten();
                println!("To join this existing cluster, use:");
                println!("  halvor join <hostname> --server={} --token={}", 
                    tailscale_hostname.as_ref().unwrap_or(&tailscale_ip),
                    existing_token);
                return Ok(());
            }
            println!();
        }
    } else {
        println!("✓ No existing cluster detected");
        println!();
    }

    // Generate token if not provided
    let cluster_token = if let Some(t) = token {
        let trimmed = t.trim();
        if trimmed.is_empty() {
            anyhow::bail!("Token cannot be empty. A token was provided but it was empty.");
        }
        println!("Using provided cluster token: {}", trimmed);
        trimmed.to_string()
    } else {
        println!("Generating cluster token...");
        let generated_token = generate_cluster_token()
            .context("Failed to generate cluster token using any available method")?;
        println!("Generated cluster token: {}", generated_token);
        generated_token
    };

    // Verify token is not empty before proceeding
    if cluster_token.is_empty() {
        anyhow::bail!(
            "Cluster token is empty after generation/assignment. This should not happen."
        );
    }

    println!("Cluster token: {}", cluster_token);
    println!();

    if !yes && !had_existing_cluster {
        print!("This will initialize a new K3s HA cluster. Continue? [y/N]: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Get Tailscale IP and hostname for cluster communication
    println!("Getting Tailscale IP for cluster communication...");
    let tailscale_ip = tailscale::get_tailscale_ip_with_fallback(&exec, hostname, config)?;

    let tailscale_hostname = tailscale::get_tailscale_hostname_remote(&exec)
        .ok()
        .flatten();

    println!("✓ Using Tailscale IP: {}", tailscale_ip);
    if let Some(ref ts_hostname) = tailscale_hostname {
        println!("✓ Using Tailscale hostname: {}", ts_hostname);
    }

    // Check for existing K3s installation and clean it up if found
    // Skip prompt if we already confirmed overwrite above
    println!("Checking for existing K3s installation...");
    let skip_cleanup_prompt = had_existing_cluster && yes;
    cleanup::cleanup_existing_k3s_with_prompt(&exec, !skip_cleanup_prompt)?;

    // Ensure halvor is installed first (the glue that enables remote operations)
    println!();
    println!("Checking for halvor (required for remote operations)...");
    tools::check_and_install_halvor(&exec)?;

    // Ensure kubectl and helm are installed
    println!();
    println!("Checking for required tools...");
    tools::check_and_install_kubectl(&exec)?;
    tools::check_and_install_helm(&exec)?;

    println!();
    println!("Installing K3s with embedded etcd...");

    // Build TLS SANs list (include Tailscale IP and hostname)
    let mut tls_sans = format!("--tls-san={}", tailscale_ip);
    if let Some(ref ts_hostname) = tailscale_hostname {
        tls_sans.push_str(&format!(" --tls-san={}", ts_hostname));
    }
    tls_sans.push_str(&format!(" --tls-san={}", hostname));

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

    // Make script executable using execute_simple (same approach as Helm)
    let chmod_output = exec.execute_simple("chmod", &["+x", remote_script_path])?;
    if !chmod_output.status.success() {
        anyhow::bail!(
            "Failed to make K3s install script executable: {}",
            String::from_utf8_lossy(&chmod_output.stderr)
        );
    }

    // Build the install command with Tailscale IP in TLS SANs
    // The K3s script handles sudo internally, but we'll run it with sudo from the start if we have the password
    // This avoids issues with the script's internal sudo detection and re-execution
    let install_cmd = if exec.get_username().ok().as_deref() == Some("root") {
        // Already running as root, no sudo needed
        format!(
            "{} server --cluster-init --token={} --disable=traefik --etcd-expose-metrics {}",
            remote_script_path, cluster_token, tls_sans
        )
    } else {
        // Not root - run with sudo to avoid script's internal sudo handling issues
        format!(
            "sudo {} server --cluster-init --token={} --disable=traefik --etcd-expose-metrics {}",
            remote_script_path, cluster_token, tls_sans
        )
    };

    // Execute the install command
    // If we have sudo password, it will be injected automatically by execute_shell_interactive
    exec.execute_shell_interactive(&install_cmd)
        .context("Failed to install K3s. The script may need sudo access - ensure passwordless sudo is configured or the script can prompt for password.")?;

    // Clean up script
    let _ = exec.execute_shell(&format!("rm -f {}", remote_script_path));

    println!();
    println!("✓ K3s installation command completed");

    // Wait for service to start and verify it's running
    println!("Waiting for K3s service to start...");
    std::thread::sleep(std::time::Duration::from_secs(10));

    // Verify K3s service is running
    println!("Verifying K3s service is running...");
    for attempt in 1..=6 {
        // Use execute_simple to check service status directly (more reliable than temp files)
        // Note: systemctl is-active doesn't require sudo, but may need it in some cases
        let status_output = exec.execute_simple("systemctl", &["is-active", "k3s"]).ok();

        let is_active = if let Some(output) = &status_output {
            let status_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let success = output.status.success();
            // systemctl is-active returns "active" if running, non-zero exit if not
            success && status_str == "active"
        } else {
            false
        };

        if is_active {
            println!("✓ K3s service is running");
            break;
        } else if attempt < 6 {
            println!(
                "  Service not active yet, waiting... (attempt {}/6)",
                attempt
            );
            std::thread::sleep(std::time::Duration::from_secs(5));
        } else {
            // Get more details about why it's not running
            let status_details = exec.execute_shell("sudo systemctl status k3s --no-pager -n 20 2>&1 || echo 'Unable to get status'").ok();
            let details = status_details
                .map(|c| String::from_utf8_lossy(&c.stdout).trim().to_string())
                .unwrap_or_else(|| "Unable to get service status".to_string());

            anyhow::bail!(
                "K3s service failed to start after {} attempts.\n\
                 Service status details:\n{}\n\
                 Check service logs: sudo journalctl -u k3s.service -n 50",
                attempt,
                details
            );
        }
    }

    // Wait for kubeconfig to be generated and API server to be ready
    println!("Waiting for kubeconfig and API server to be ready...");
    println!("  (This may take 1-2 minutes for embedded etcd to initialize)");

    let max_wait_attempts = 24; // 24 * 5 seconds = 2 minutes max
    let mut api_ready = false;

    for attempt in 1..=max_wait_attempts {
        // Check if kubeconfig exists and API server is responding
        let kubeconfig_check = exec
            .execute_shell(
                "sudo test -f /etc/rancher/k3s/k3s.yaml && echo 'exists' || echo 'missing'",
            )
            .ok();
        let kubeconfig_exists = kubeconfig_check
            .map(|c| String::from_utf8_lossy(&c.stdout).trim() == "exists")
            .unwrap_or(false);

        if kubeconfig_exists {
            // Try to query the API server to see if it's ready
            let api_check = exec
                .execute_shell("sudo k3s kubectl get nodes --request-timeout=5s 2>&1 | head -1")
                .ok();
            if let Some(api_output) = api_check {
                let api_str = String::from_utf8_lossy(&api_output.stdout);
                // If we get a response (even if it's empty or an error), API server is likely up
                // Check for common "not ready" errors
                if !api_str.contains("Unable to connect")
                    && !api_str.contains("connection refused")
                    && !api_str.contains("dial tcp")
                    && api_output.status.code() != Some(1)
                {
                    api_ready = true;
                    break;
                }
            }
        }

        if attempt < max_wait_attempts {
            if attempt % 6 == 0 {
                // Print progress every 30 seconds
                println!("  Still waiting... ({} seconds elapsed)", attempt * 5);
            }
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    }

    if !api_ready {
        println!("⚠ Warning: API server may not be fully ready yet.");
        println!("  The cluster is still initializing. You can check status with:");
        println!("  halvor k3s status -H {}", hostname);
        println!("  Or wait a few more minutes and try joining nodes.");
    } else {
        println!("✓ API server is ready");
    }

    println!();
    println!("✓ K3s initialized successfully!");
    println!();

    // Ensure token is not empty before printing
    if cluster_token.is_empty() {
        anyhow::bail!(
            "Cluster token is empty - this should not happen. Token generation may have failed."
        );
    }

    println!("Save this token to join additional nodes:");
    println!("  K3S_TOKEN={}", cluster_token);
    println!();
    println!("Join additional control plane nodes with:");
    let server_addr = tailscale_hostname.as_ref().unwrap_or(&tailscale_ip);
    println!(
        "  halvor k3s join --server={} --token={}",
        server_addr, cluster_token
    );
    println!();

    // Setup halvor agent service on the primary node
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Setting up halvor agent service on primary node");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    
    let exec = Executor::new(hostname, config)?;
    let web_port = if std::env::var("HALVOR_WEB_DIR").is_ok() {
        Some(13000)
    } else {
        None
    };
    
    if let Err(e) = agent_service::setup_agent_service(&exec, web_port) {
        eprintln!("⚠️  Warning: Failed to setup halvor agent service: {}", e);
        eprintln!("   You can set it up manually later with: halvor agent start --port 13500 --daemon");
    } else {
        println!("✓ Halvor agent service is running on {}", hostname);
        println!("  Agent API: port 13500 (over Tailscale)");
        if web_port.is_some() {
            println!("  Web UI: port 13000 (over Tailscale)");
        }
    }
    println!();

    Ok(())
}
