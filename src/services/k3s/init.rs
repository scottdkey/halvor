//! K3s cluster initialization

use crate::config::EnvConfig;
use crate::services::k3s::utils::generate_cluster_token;
use crate::services::k3s::{cleanup, tools};
use crate::services::tailscale;
use crate::utils::exec::{CommandExecutor, Executor};
use anyhow::{Context, Result};
use std::io::{self, Write};

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

    if !yes {
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
    println!("Checking for existing K3s installation...");
    cleanup::cleanup_existing_k3s(&exec)?;

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

    // Write script to remote host
    let remote_script_path = "/tmp/k3s-install.sh";
    exec.write_file(remote_script_path, k3s_script.as_bytes())
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
    let install_cmd = format!(
        "{} server --cluster-init --token={} --disable=traefik --etcd-expose-metrics {}",
        remote_script_path, cluster_token, tls_sans
    );

    exec.execute_shell_interactive(&install_cmd)
        .context("Failed to install K3s")?;

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

    Ok(())
}
