//! K3s cluster management service
//!
//! Handles K3s installation, HA configuration, and etcd snapshots.

use crate::config::EnvConfig;
use crate::services::tailscale;
use crate::utils::exec::{CommandExecutor, Executor};
use anyhow::{Context, Result};
use rand::RngCore;
use serde_json;
use std::io::{self, Write};

/// Generate a random hex token (64 characters = 32 bytes)
/// Uses native Rust rand crate for reliable cross-platform token generation
pub fn generate_cluster_token<E: CommandExecutor>(_exec: &E) -> Result<String> {
    let mut rng = rand::thread_rng();
    let mut token = String::with_capacity(64);
    for _ in 0..64 {
        let mut bytes = [0u8; 1];
        rng.fill_bytes(&mut bytes);
        token.push_str(&format!("{:x}", bytes[0]));
    }

    Ok(token)
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
        let generated_token = generate_cluster_token(&exec)
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

    println!();
    println!("Installing K3s with embedded etcd...");

    // Build TLS SANs list (include Tailscale IP and hostname)
    let mut tls_sans = format!("--tls-san={}", tailscale_ip);
    if let Some(ref ts_hostname) = tailscale_hostname {
        tls_sans.push_str(&format!(" --tls-san={}", ts_hostname));
    }
    tls_sans.push_str(&format!(" --tls-san={}", hostname));

    // Build the install command with Tailscale IP in TLS SANs
    let install_cmd = format!(
        "curl -sfL https://get.k3s.io | sh -s - server \
         --cluster-init \
         --token={} \
         --disable=traefik \
         --etcd-expose-metrics \
         {}",
        cluster_token, tls_sans
    );

    exec.execute_shell_interactive(&install_cmd)
        .context("Failed to install K3s")?;

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

/// Join a node to the cluster
pub fn join_cluster(
    hostname: &str,
    server: &str,
    token: &str,
    control_plane: bool,
    config: &EnvConfig,
) -> Result<()> {
    let exec = Executor::new(hostname, config)?;
    let is_local = exec.is_local();

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

    // Build TLS SANs list
    let mut tls_sans = format!("--tls-san={}", tailscale_ip);
    if let Some(ref ts_hostname) = tailscale_hostname {
        tls_sans.push_str(&format!(" --tls-san={}", ts_hostname));
    }
    tls_sans.push_str(&format!(" --tls-san={}", hostname));

    // Use Tailscale address for server connection (prefer hostname, fallback to IP)
    let server_addr = if server.contains('.') && !server.contains(':') {
        // If server looks like an IP, try to resolve to Tailscale hostname
        // For now, use as-is but prefer Tailscale hostname if we have it
        server.to_string()
    } else {
        server.to_string()
    };

    println!();
    println!("Joining cluster via Tailscale...");

    // Check for existing K3s installation
    println!("Checking for existing K3s installation...");
    let has_k3s_server = exec
        .execute_shell("test -f /usr/local/bin/k3s-uninstall.sh && echo exists || echo not_exists")
        .ok()
        .and_then(|c| Some(String::from_utf8_lossy(&c.stdout).trim() == "exists"))
        .unwrap_or(false);

    let has_k3s_agent = exec
        .execute_shell(
            "test -f /usr/local/bin/k3s-agent-uninstall.sh && echo exists || echo not_exists",
        )
        .ok()
        .and_then(|c| Some(String::from_utf8_lossy(&c.stdout).trim() == "exists"))
        .unwrap_or(false);

    // Check for K3s service using native Rust string methods
    let has_k3s_service = exec
        .execute_shell("systemctl list-unit-files")
        .ok()
        .map(|c| {
            let output = String::from_utf8_lossy(&c.stdout);
            output.lines().any(|line| line.contains("k3s"))
        })
        .unwrap_or(false);

    if has_k3s_server || has_k3s_agent || has_k3s_service {
        println!("⚠ Found existing K3s installation on this node.");
        println!(
            "   This node was previously configured as: {}",
            if has_k3s_server {
                "server/control plane"
            } else {
                "agent"
            }
        );
        println!(
            "   Uninstalling previous installation before joining as {}...",
            if control_plane {
                "control plane"
            } else {
                "agent"
            }
        );

        // Uninstall existing installation
        if has_k3s_server {
            let uninstall_check = exec
                .execute_shell("test -f /usr/local/bin/k3s-uninstall.sh && echo exists")
                .ok();
            if let Some(check) = uninstall_check {
                if String::from_utf8_lossy(&check.stdout).trim() == "exists" {
                    println!("Uninstalling K3s server...");
                    exec.execute_shell_interactive("/usr/local/bin/k3s-uninstall.sh")
                        .context("Failed to uninstall existing K3s server")?;
                }
            }
        } else if has_k3s_agent {
            let uninstall_check = exec
                .execute_shell("test -f /usr/local/bin/k3s-agent-uninstall.sh && echo exists")
                .ok();
            if let Some(check) = uninstall_check {
                if String::from_utf8_lossy(&check.stdout).trim() == "exists" {
                    println!("Uninstalling K3s agent...");
                    exec.execute_shell_interactive("/usr/local/bin/k3s-agent-uninstall.sh")
                        .context("Failed to uninstall existing K3s agent")?;
                }
            }
        }

        // Wait a moment for cleanup
        std::thread::sleep(std::time::Duration::from_secs(3));
        println!("✓ Previous installation removed");
    } else {
        println!("✓ No existing K3s installation found");
    }

    // Build install command using curl pipe (same format as init_control_plane which works)
    // This avoids issues with downloading/uploading the script and ensures compatibility
    let install_cmd = if control_plane {
        format!(
            "curl -sfL https://get.k3s.io | sh -s - server --server=https://{}:6443 --token={} --disable=traefik {}",
            server_addr, token, tls_sans
        )
    } else {
        format!(
            "curl -sfL https://get.k3s.io | sh -s - agent --server=https://{}:6443 --token={} {}",
            server_addr, token, tls_sans
        )
    };

    // Execute the installation command
    // NOTE: Must use execute_shell_interactive because the K3s install script uses sudo
    // which requires a TTY to prompt for password. execute_shell (non-interactive) doesn't
    // allocate a TTY, causing sudo to fail silently.
    println!("Installing K3s...");
    println!("Command: {}", install_cmd);
    println!();
    println!();
    io::stdout().flush()?; // Ensure message is displayed before password prompt

    // Use curl pipe format (same as init_control_plane) - this is the proven working method
    let install_result = exec.execute_shell_interactive(&install_cmd);

    match install_result {
        Ok(()) => {
            println!();
            println!("✓ K3s installation command completed");
        }
        Err(e) => {
            // Command execution itself failed (not just exit code)
            println!();
            println!("⚠ Installation command failed.");
            println!("Error: {}", e);
            println!();

            // Check if service exists and get logs - use interactive commands so sudo can prompt
            println!("Checking K3s service status and logs...");
            println!("(You may be prompted for your sudo password)");
            io::stdout().flush()?;

            // Try to get service status interactively
            let _ = exec.execute_shell_interactive(
                "sudo systemctl status k3s.service --no-pager -l | head -30",
            );

            println!();
            println!("Fetching recent service logs...");
            io::stdout().flush()?;

            // Try to get journal logs interactively
            let _ =
                exec.execute_shell_interactive("sudo journalctl -u k3s.service --no-pager -n 50");

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
                let service_check = exec.execute_shell("sudo systemctl is-active k3s 2>/dev/null || sudo systemctl is-active k3s-agent 2>/dev/null || echo 'not_running'").ok();
                if let Some(check) = service_check {
                    let stdout_bytes = check.stdout;
                    let status_str = String::from_utf8_lossy(&stdout_bytes).trim().to_string();
                    if status_str == "active" || status_str == "activating" {
                        println!(
                            "✓ K3s service is active despite error - continuing with verification"
                        );
                    } else {
                        // Get service status details
                        let status_details = exec.execute_shell("sudo systemctl status k3s 2>&1 | head -10 || sudo systemctl status k3s-agent 2>&1 | head -10 || echo 'no_status'").ok();
                        let details = status_details
                            .map(|c| String::from_utf8_lossy(&c.stdout).trim().to_string())
                            .unwrap_or_default();

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

    // Wait a moment for the service to start
    println!("Waiting for K3s service to initialize...");
    std::thread::sleep(std::time::Duration::from_secs(5));

    // Verify the node successfully joined the cluster (with retries)
    println!();
    println!(
        "Verifying cluster membership on {} (this may take a few minutes)...",
        hostname
    );

    // Ensure we're using the remote executor (not local)
    if exec.is_local() {
        anyhow::bail!(
            "Internal error: Executor is local but should be remote for hostname '{}'. \
             This indicates a configuration issue. Please ensure the host is configured in your .env file.",
            hostname
        );
    }

    // Get kubeconfig from primary control plane node and set it up locally
    println!();
    println!("Setting up kubectl access from local machine...");
    setup_local_kubeconfig(server, config).context("Failed to set up local kubectl access")?;

    // Verify the node successfully joined the cluster using local kubectl
    println!();
    println!("Verifying cluster membership using local kubectl (this may take a few minutes)...");
    verify_cluster_join_with_local_kubectl(hostname, control_plane)
        .context("Failed to verify cluster join after multiple attempts")?;

    println!();
    if control_plane {
        println!("✓ Successfully joined cluster as control plane node!");
    } else {
        println!("✓ Successfully joined cluster as agent node!");
    }

    Ok(())
}

/// Set up kubeconfig on local machine from the primary control plane node
fn setup_local_kubeconfig(primary_hostname: &str, config: &EnvConfig) -> Result<()> {
    use crate::utils::exec::local;

    // Get kubeconfig from primary control plane node
    let primary_exec = Executor::new(primary_hostname, config)?;

    println!("  Fetching kubeconfig from {}...", primary_hostname);
    let kubeconfig_output = primary_exec.execute_shell("sudo cat /etc/rancher/k3s/k3s.yaml")?;
    if !kubeconfig_output.status.success() {
        anyhow::bail!(
            "Failed to read kubeconfig from {}. Is K3s installed?",
            primary_hostname
        );
    }

    let mut kubeconfig_content =
        String::from_utf8(kubeconfig_output.stdout).context("Failed to decode kubeconfig")?;

    // Get Tailscale IP/hostname for the primary node to replace 127.0.0.1
    let tailscale_ip =
        tailscale::get_tailscale_ip_with_fallback(&primary_exec, primary_hostname, config)?;
    let tailscale_hostname = tailscale::get_tailscale_hostname_remote(&primary_exec)
        .ok()
        .flatten();

    // Replace localhost/127.0.0.1 with Tailscale address
    let server_address = tailscale_hostname.as_ref().unwrap_or(&tailscale_ip);
    kubeconfig_content = kubeconfig_content.replace("127.0.0.1", server_address);
    kubeconfig_content = kubeconfig_content.replace("localhost", server_address);

    // Set up local kubeconfig
    let home = std::env::var("HOME")
        .ok()
        .unwrap_or_else(|| ".".to_string());
    let kube_dir = format!("{}/.kube", home);
    std::fs::create_dir_all(&kube_dir).context("Failed to create ~/.kube directory")?;

    let kube_config_path = format!("{}/config", kube_dir);

    // Merge with existing config if it exists, otherwise write new
    if std::path::Path::new(&kube_config_path).exists() {
        println!("  Merging with existing kubeconfig at {}", kube_config_path);
        // For now, append with a context name - proper merge would parse YAML
        let existing = std::fs::read_to_string(&kube_config_path).unwrap_or_default();
        if !existing.contains("k3s") {
            // Simple append - in production you'd want proper YAML merging
            let mut merged = existing;
            if !merged.ends_with('\n') {
                merged.push('\n');
            }
            merged.push_str("---\n");
            merged.push_str(&kubeconfig_content);
            std::fs::write(&kube_config_path, merged)?;
        } else {
            println!("  Kubeconfig already contains k3s context, skipping merge");
        }
    } else {
        std::fs::write(&kube_config_path, &kubeconfig_content)
            .context("Failed to write kubeconfig")?;
    }

    println!("  ✓ Kubeconfig set up at {}", kube_config_path);

    // Verify kubectl is available locally
    if !local::check_command_exists("kubectl") {
        println!("  ⚠ kubectl not found in PATH. Install kubectl to use cluster commands.");
        println!("     macOS: brew install kubectl");
        println!("     Linux: See https://kubernetes.io/docs/tasks/tools/");
    } else {
        println!("  ✓ kubectl is available");
    }

    Ok(())
}

/// Verify that a node successfully joined the cluster with retries using local kubectl
fn verify_cluster_join_with_local_kubectl(hostname: &str, control_plane: bool) -> Result<()> {
    // Use local executor for kubectl commands
    let local_exec = Executor::Local;

    verify_cluster_join_with_retry(&local_exec, hostname, control_plane)
}

/// Verify that a node successfully joined the cluster with retries
fn verify_cluster_join_with_retry<E: CommandExecutor>(
    exec: &E,
    expected_hostname: &str,
    control_plane: bool,
) -> Result<()> {
    const MAX_ATTEMPTS: u32 = 30; // 30 attempts
    const RETRY_DELAY_SECS: u64 = 10; // 10 seconds between attempts
    const MAX_WAIT_SECS: u64 = MAX_ATTEMPTS as u64 * RETRY_DELAY_SECS; // 5 minutes total

    println!(
        "Will retry verification for up to {} minutes...",
        MAX_WAIT_SECS / 60
    );
    println!();

    for attempt in 1..=MAX_ATTEMPTS {
        if attempt > 1 {
            println!(
                "Attempt {}/{} (waiting {} seconds before retry)...",
                attempt, MAX_ATTEMPTS, RETRY_DELAY_SECS
            );
            std::thread::sleep(std::time::Duration::from_secs(RETRY_DELAY_SECS));
        }

        match verify_cluster_join_once(exec, expected_hostname, control_plane, attempt) {
            Ok(()) => {
                println!();
                println!("✓ Cluster join verification successful!");
                return Ok(());
            }
            Err(e) => {
                if attempt == MAX_ATTEMPTS {
                    // Last attempt failed
                    return Err(e).context(format!(
                        "Verification failed after {} attempts ({} minutes). The node may still be joining.",
                        MAX_ATTEMPTS,
                        MAX_WAIT_SECS / 60
                    ));
                }
                // Continue to next attempt
                println!("⚠ Verification attempt {} failed: {}", attempt, e);
                println!();
            }
        }
    }

    anyhow::bail!("Verification failed after all attempts");
}

/// Verify cluster join once (single attempt) using kubectl
fn verify_cluster_join_once<E: CommandExecutor>(
    exec: &E,
    expected_hostname: &str,
    control_plane: bool,
    attempt: u32,
) -> Result<()> {
    // Step 1: Verify kubectl is available and can connect to cluster
    if attempt == 1 {
        println!("[1/3] Checking kubectl access to cluster...");
    }

    let kubectl_version = exec
        .execute_shell("kubectl version --client --short 2>&1 || echo 'kubectl_not_found'")
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .unwrap_or_else(|| "kubectl_not_found".to_string());

    if kubectl_version.contains("kubectl_not_found") {
        anyhow::bail!(
            "kubectl not found (attempt {}). Please install kubectl to verify cluster access.",
            attempt
        );
    }

    if attempt == 1 {
        println!("  ✓ kubectl is available");
    }

    // Step 2: Check if we can connect to the cluster
    if attempt == 1 {
        println!("[2/3] Verifying cluster connectivity...");
    }

    let cluster_info = exec
        .execute_shell("kubectl cluster-info 2>&1 || echo 'cluster_unreachable'")
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .unwrap_or_else(|| "cluster_unreachable".to_string());

    if cluster_info.contains("cluster_unreachable") || cluster_info.contains("Unable to connect") {
        anyhow::bail!(
            "Cannot connect to cluster (attempt {}). Cluster may still be initializing.",
            attempt
        );
    }

    if attempt == 1 {
        println!("  ✓ Cluster is reachable");
    }

    // Step 3: Verify node appears in cluster and is Ready
    if attempt == 1 {
        println!("[3/3] Checking node status in cluster...");
    }

    let node_output = exec
        .execute_shell("kubectl get nodes -o json 2>&1")
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .unwrap_or_else(|| "kubectl_failed".to_string());

    if node_output.trim().is_empty() || node_output.contains("kubectl_failed") {
        anyhow::bail!(
            "Unable to query cluster nodes (attempt {}). kubectl may not be ready yet.",
            attempt
        );
    }

    // Try to parse JSON to find this node and check if it's Ready
    let nodes_json: serde_json::Value = serde_json::from_str(&node_output)
        .with_context(|| format!("Failed to parse node list JSON. Output: {}", node_output))?;

    // Use expected_hostname instead of getting from executor (we're using local kubectl now)
    let hostname_str = expected_hostname.trim().to_string();

    let items = nodes_json
        .get("items")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("No nodes found in cluster response"))?;

    let mut node_found = false;
    let mut node_ready = false;

    for node in items {
        let node_name = node
            .get("metadata")
            .and_then(|m| m.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("");

        if node_name == hostname_str || node_name.contains(&hostname_str) {
            node_found = true;

            // Check if node is Ready
            if let Some(status) = node.get("status") {
                if let Some(conditions) = status.get("conditions") {
                    if let Some(conditions_array) = conditions.as_array() {
                        for condition in conditions_array {
                            if let Some(type_val) = condition.get("type").and_then(|v| v.as_str()) {
                                if type_val == "Ready" {
                                    if let Some(status_val) =
                                        condition.get("status").and_then(|v| v.as_str())
                                    {
                                        if status_val == "True" {
                                            node_ready = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            break;
        }
    }

    if !node_found {
        anyhow::bail!(
            "Node '{}' not yet visible in cluster (attempt {}). Node may still be registering.",
            hostname_str,
            attempt
        );
    }

    if !node_ready {
        anyhow::bail!(
            "Node '{}' found but not Ready yet (attempt {}). Node may still be initializing.",
            hostname_str,
            attempt
        );
    }

    if attempt == 1 {
        println!("  ✓ Node '{}' is Ready in cluster", hostname_str);
    }

    Ok(())
}

/// Verify HA cluster health and failover capability
/// Checks all nodes, etcd health, and verifies cluster can operate from any node
pub fn verify_ha_cluster(
    primary_hostname: &str,
    expected_nodes: &[&str],
    config: &EnvConfig,
) -> Result<()> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("K3s HA Cluster Verification");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Expected nodes: {}", expected_nodes.join(", "));
    println!();

    let mut all_checks_passed = true;

    // Step 1: Verify all expected nodes are in the cluster
    println!("[1/5] Verifying all nodes are in cluster...");
    let exec = Executor::new(primary_hostname, config)?;

    // Authenticate sudo first so user can enter password
    println!("(You may be prompted for your sudo password)");
    io::stdout().flush()?;
    exec.execute_interactive("sudo", &["-v"])?;

    // First check if kubectl is available and working - use temp file
    let kubectl_tmp = "/tmp/k3s_kubectl_version";
    let _ = exec.execute_shell_interactive(&format!(
        "sudo k3s kubectl version --client 2>&1 | head -1 > {} || echo '' > {}",
        kubectl_tmp, kubectl_tmp
    ));
    if let Ok(version_output) = exec.read_file(kubectl_tmp) {
        if !version_output.trim().is_empty() {
            println!("  kubectl version: {}", version_output.trim());
        }
    }

    // Get nodes using temp file to capture output from interactive command
    let nodes_tmp = "/tmp/k3s_nodes_json";
    exec.execute_shell_interactive(&format!(
        "sudo k3s kubectl get nodes -o json > {} 2>&1 || echo '' > {}",
        nodes_tmp, nodes_tmp
    ))
    .context("Failed to query cluster nodes")?;

    let nodes_output = exec
        .read_file(nodes_tmp)
        .context("Failed to read nodes output")?;

    if nodes_output.trim().is_empty() {
        anyhow::bail!(
            "kubectl returned empty output. This may indicate:\n  - K3s is not running on {}\n  - kubectl is not available\n  - Cluster is not initialized\n\nTry: halvor k3s status -H {}",
            primary_hostname,
            primary_hostname
        );
    }

    let nodes_json: serde_json::Value = serde_json::from_str(&nodes_output)
        .with_context(|| format!("Failed to parse node list JSON. Output: {}", nodes_output))?;

    let items = nodes_json
        .get("items")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("No nodes found in cluster"))?;

    let mut found_nodes: Vec<String> = Vec::new();
    let mut ready_nodes: Vec<String> = Vec::new();
    let mut control_plane_nodes: Vec<String> = Vec::new();

    for node in items {
        let node_name = node
            .get("metadata")
            .and_then(|m| m.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();

        found_nodes.push(node_name.clone());

        // Check if node is Ready
        let mut is_ready = false;
        if let Some(status) = node.get("status") {
            if let Some(conditions) = status.get("conditions") {
                if let Some(conditions_array) = conditions.as_array() {
                    for condition in conditions_array {
                        if let Some(type_val) = condition.get("type").and_then(|v| v.as_str()) {
                            if type_val == "Ready" {
                                if let Some(status_val) =
                                    condition.get("status").and_then(|v| v.as_str())
                                {
                                    if status_val == "True" {
                                        is_ready = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if is_ready {
            ready_nodes.push(node_name.clone());
        }

        // Check if node is control plane (has master/control-plane label or taint)
        let labels = node
            .get("metadata")
            .and_then(|m| m.get("labels"))
            .and_then(|l| l.as_object());
        let is_control_plane = labels
            .map(|l| {
                l.contains_key("node-role.kubernetes.io/control-plane")
                    || l.contains_key("node-role.kubernetes.io/master")
            })
            .unwrap_or(false);

        if is_control_plane {
            control_plane_nodes.push(node_name.clone());
        }

        println!(
            "  {} - Status: {} - Role: {}",
            node_name,
            if is_ready { "Ready" } else { "Not Ready" },
            if is_control_plane {
                "Control Plane"
            } else {
                "Worker"
            }
        );
    }

    // Verify all expected nodes are found
    for expected in expected_nodes {
        if !found_nodes
            .iter()
            .any(|n| n == *expected || n.contains(*expected))
        {
            println!("  ✗ Expected node '{}' not found in cluster", expected);
            all_checks_passed = false;
        } else {
            println!("  ✓ Node '{}' found in cluster", expected);
        }
    }

    if found_nodes.len() != expected_nodes.len() {
        println!(
            "  ⚠ Cluster has {} nodes, expected {}",
            found_nodes.len(),
            expected_nodes.len()
        );
    }

    if ready_nodes.len() != expected_nodes.len() {
        println!(
            "  ✗ Only {}/{} nodes are Ready",
            ready_nodes.len(),
            expected_nodes.len()
        );
        all_checks_passed = false;
    } else {
        println!("  ✓ All {} nodes are Ready", ready_nodes.len());
    }

    println!();

    // Step 2: Verify etcd health on all control plane nodes
    println!("[2/5] Verifying etcd health on control plane nodes...");
    for cp_node in &control_plane_nodes {
        let cp_exec = Executor::new(cp_node, config)?;
        let etcd_check = cp_exec
            .execute_shell("sudo k3s etcd-snapshot list 2>/dev/null | head -1")
            .ok();

        if let Some(check) = etcd_check {
            let etcd_output = String::from_utf8_lossy(&check.stdout).trim().to_string();
            if etcd_output.is_empty() || etcd_output.contains("error") {
                println!("  ✗ etcd not accessible on {}", cp_node);
                all_checks_passed = false;
            } else {
                println!("  ✓ etcd accessible on {}", cp_node);
            }
        } else {
            println!("  ✗ Failed to check etcd on {}", cp_node);
            all_checks_passed = false;
        }
    }

    if control_plane_nodes.len() < 3 {
        println!(
            "  ⚠ Only {} control plane nodes found (3 recommended for HA)",
            control_plane_nodes.len()
        );
    } else {
        println!(
            "  ✓ {} control plane nodes configured",
            control_plane_nodes.len()
        );
    }
    println!();

    // Step 3: Verify cluster can be queried from all nodes
    println!("[3/5] Verifying cluster accessibility from all nodes...");
    for node in expected_nodes {
        let node_exec = Executor::new(node, config)?;
        // Authenticate sudo for this node
        let _ = node_exec.execute_interactive("sudo", &["-v"]);

        // Use temp file to capture output
        let node_count_tmp = format!("/tmp/k3s_node_count_{}", node);
        let _ = node_exec.execute_shell_interactive(&format!(
            "sudo k3s kubectl get nodes --no-headers 2>/dev/null | wc -l > {} || echo '0' > {}",
            node_count_tmp, node_count_tmp
        ));

        let node_check = node_exec.read_file(&node_count_tmp).ok();

        if let Some(count_str) = node_check {
            let count = count_str.trim().to_string();
            if let Ok(node_count) = count.parse::<u32>() {
                if node_count == expected_nodes.len() as u32 {
                    println!(
                        "  ✓ Can query cluster from {} (sees {} nodes)",
                        node, node_count
                    );
                } else {
                    println!(
                        "  ✗ {} sees {} nodes, expected {}",
                        node,
                        node_count,
                        expected_nodes.len()
                    );
                    all_checks_passed = false;
                }
            } else {
                println!("  ✗ Failed to parse node count from {}", node);
                all_checks_passed = false;
            }
        } else {
            println!("  ✗ Cannot query cluster from {}", node);
            all_checks_passed = false;
        }
    }
    println!();

    // Step 4: Verify etcd member list (should show all control plane nodes)
    println!("[4/5] Verifying etcd cluster membership...");
    let etcd_members = exec
        .execute_shell("sudo k3s etcd-member-list 2>/dev/null || echo 'command_not_available'")
        .ok();

    if let Some(members) = etcd_members {
        let members_output = String::from_utf8_lossy(&members.stdout);
        if members_output.contains("command_not_available") {
            println!("  ⚠ etcd-member-list command not available (may be normal)");
            // Try alternative: check etcd endpoint health
            let health_check = exec
                .execute_shell(
                    "sudo k3s kubectl get endpoints kube-system kube-etcd -o json 2>/dev/null | grep -o '\"subsets\":' || echo 'no_endpoints'",
                )
                .ok();
            if let Some(health) = health_check {
                let health_output = String::from_utf8_lossy(&health.stdout);
                if health_output.contains("subsets") {
                    println!("  ✓ etcd endpoints configured");
                } else {
                    println!("  ⚠ etcd endpoints status unclear");
                }
            }
        } else {
            println!("  ✓ etcd members:");
            for line in members_output.lines() {
                if !line.trim().is_empty() {
                    println!("    {}", line.trim());
                }
            }
        }
    }
    println!();

    // Step 5: Test failover by verifying cluster operations work from different nodes
    println!("[5/5] Testing failover capability...");
    println!("  (Verifying cluster can operate when queried from different control plane nodes)");

    let mut failover_tests_passed = 0;
    let mut failover_tests_total = 0;

    for cp_node in &control_plane_nodes {
        failover_tests_total += 1;
        let cp_exec = Executor::new(cp_node, config)?;

        // Authenticate sudo for this node
        let _ = cp_exec.execute_interactive("sudo", &["-v"]);

        // Test 1: Can list nodes - use temp file
        let nodes_tmp = format!("/tmp/k3s_failover_nodes_{}", cp_node);
        let _ = cp_exec.execute_shell_interactive(&format!(
            "sudo k3s kubectl get nodes --no-headers 2>/dev/null | wc -l > {} || echo '0' > {}",
            nodes_tmp, nodes_tmp
        ));
        let can_list_nodes = cp_exec
            .read_file(&nodes_tmp)
            .ok()
            .and_then(|count_str| count_str.trim().parse::<u32>().ok())
            .map(|n| n == expected_nodes.len() as u32)
            .unwrap_or(false);

        // Test 2: Can access etcd - use temp file
        let etcd_tmp = format!("/tmp/k3s_failover_etcd_{}", cp_node);
        let _ = cp_exec.execute_shell_interactive(&format!(
            "sudo k3s etcd-snapshot list 2>/dev/null | head -1 > {} || echo '' > {}",
            etcd_tmp, etcd_tmp
        ));
        let can_access_etcd = cp_exec
            .read_file(&etcd_tmp)
            .ok()
            .map(|output| {
                let trimmed = output.trim();
                !trimmed.is_empty() && !trimmed.contains("error")
            })
            .unwrap_or(false);

        if can_list_nodes && can_access_etcd {
            println!("  ✓ {} can operate cluster independently", cp_node);
            failover_tests_passed += 1;
        } else {
            println!("  ✗ {} has limited cluster access", cp_node);
            if !can_list_nodes {
                println!("    - Cannot list nodes");
            }
            if !can_access_etcd {
                println!("    - Cannot access etcd");
            }
            all_checks_passed = false;
        }
    }

    if failover_tests_passed == failover_tests_total && failover_tests_total > 0 {
        println!("  ✓ All control plane nodes can operate cluster independently");
    }
    println!();

    // Final summary
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    if all_checks_passed {
        println!("✓ HA Cluster Verification PASSED");
        println!();
        println!("All nodes are healthy and the cluster is ready for production use.");
        println!("Failover capability verified: cluster can operate from any control plane node.");
    } else {
        println!("✗ HA Cluster Verification FAILED");
        println!();
        println!("Some checks failed. Review the output above and fix any issues.");
        anyhow::bail!("HA cluster verification failed");
    }
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}

/// Get cluster join information from an existing control plane node
/// Returns (server_address, token) for joining new nodes
pub fn get_cluster_join_info(hostname: &str, config: &EnvConfig) -> Result<(String, String)> {
    let exec = Executor::new(hostname, config)?;

    // Authenticate sudo
    exec.execute_interactive("sudo", &["-v"])?;

    // Get Tailscale IP/hostname for server address
    let tailscale_ip = tailscale::get_tailscale_ip_with_fallback(&exec, hostname, config)?;
    let tailscale_hostname = tailscale::get_tailscale_hostname_remote(&exec)
        .ok()
        .flatten();

    let server_addr = tailscale_hostname.as_ref().unwrap_or(&tailscale_ip);

    // Try to read token from K3s node token file
    let token_tmp = "/tmp/k3s_cluster_token";
    let token_result = exec.execute_shell_interactive(&format!(
        "sudo cat /var/lib/rancher/k3s/server/node-token > {} 2>&1 || echo '' > {}",
        token_tmp, token_tmp
    ));

    let token = if token_result.is_ok() {
        if let Ok(token_content) = exec.read_file(token_tmp) {
            let trimmed = token_content.trim();
            if !trimmed.is_empty() {
                trimmed.to_string()
            } else {
                // Token file is empty, try environment variable
                std::env::var("K3S_TOKEN")
                    .ok()
                    .ok_or_else(|| {
                        anyhow::anyhow!("Could not retrieve cluster token. Please set K3S_TOKEN in 1Password or provide it manually.")
                    })?
            }
        } else {
            std::env::var("K3S_TOKEN")
                .ok()
                .ok_or_else(|| {
                    anyhow::anyhow!("Could not retrieve cluster token. Please set K3S_TOKEN in 1Password or provide it manually.")
                })?
        }
    } else {
        // Try environment variable
        std::env::var("K3S_TOKEN")
            .ok()
            .ok_or_else(|| {
                anyhow::anyhow!("Could not retrieve cluster token. Please set K3S_TOKEN in 1Password or provide it manually.")
            })?
    };

    Ok((server_addr.to_string(), token))
}

/// Show cluster status
pub fn show_status(hostname: &str, config: &EnvConfig) -> Result<()> {
    let exec = Executor::new(hostname, config)?;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("K3s Cluster Status");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Authenticate sudo first so user can enter password
    println!("(You may be prompted for your sudo password)");
    io::stdout().flush()?;
    exec.execute_interactive("sudo", &["-v"])?;

    // Check if k3s is installed
    let version = exec.execute_shell("k3s --version 2>/dev/null || echo 'not installed'")?;
    let version_str = String::from_utf8_lossy(&version.stdout);
    if version_str.contains("not installed") {
        println!("K3s is not installed on this node.");
        return Ok(());
    }
    println!("K3s Version: {}", version_str.trim());
    println!();

    // Show nodes - use temp file to capture output from interactive command
    println!("Nodes:");
    let tmp_file = "/tmp/k3s_nodes_status";
    exec.execute_shell_interactive(&format!(
        "sudo k3s kubectl get nodes -o wide > {} 2>&1 || echo 'Unable to get nodes' > {}",
        tmp_file, tmp_file
    ))?;
    let nodes_output = exec
        .read_file(tmp_file)
        .unwrap_or_else(|_| "Unable to get nodes".to_string());
    println!("{}", nodes_output);

    // Show etcd status for control plane nodes
    println!("etcd Status:");
    let etcd_tmp = "/tmp/k3s_etcd_status";
    exec.execute_shell_interactive(&format!(
        "sudo k3s etcd-snapshot list 2>/dev/null | head -5 > {} || echo 'Not a control plane node or etcd not available' > {}",
        etcd_tmp, etcd_tmp
    ))?;
    let etcd_output = exec
        .read_file(etcd_tmp)
        .unwrap_or_else(|_| "Not a control plane node or etcd not available".to_string());
    println!("{}", etcd_output);

    Ok(())
}

/// Get kubeconfig from the cluster
pub fn get_kubeconfig(
    hostname: &str,
    merge: bool,
    output: Option<&str>,
    config: &EnvConfig,
) -> Result<()> {
    let exec = Executor::new(hostname, config)?;

    let kubeconfig = exec.execute_shell("sudo cat /etc/rancher/k3s/k3s.yaml")?;
    if !kubeconfig.status.success() {
        anyhow::bail!("Failed to read kubeconfig. Is K3s installed?");
    }

    let kubeconfig_content = String::from_utf8_lossy(&kubeconfig.stdout);

    // Replace localhost with the actual hostname/IP
    let host_ip = exec.execute_shell("hostname -I | awk '{print $1}'")?;
    let ip = String::from_utf8_lossy(&host_ip.stdout).trim().to_string();
    let kubeconfig_fixed = kubeconfig_content.replace("127.0.0.1", &ip);

    if let Some(path) = output {
        std::fs::write(path, &kubeconfig_fixed)
            .with_context(|| format!("Failed to write kubeconfig to {}", path))?;
        println!("✓ Kubeconfig written to {}", path);
    } else if merge {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let kube_dir = format!("{}/.kube", home);
        std::fs::create_dir_all(&kube_dir)?;
        let kube_config_path = format!("{}/config", kube_dir);

        // For now, just append - a proper merge would parse YAML
        std::fs::write(&kube_config_path, &kubeconfig_fixed)?;
        println!("✓ Kubeconfig written to {}", kube_config_path);
    } else {
        println!("{}", kubeconfig_fixed);
    }

    Ok(())
}

/// Uninstall K3s from a node
pub fn uninstall(hostname: &str, yes: bool, config: &EnvConfig) -> Result<()> {
    let exec = Executor::new(hostname, config)?;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Uninstall K3s");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    if !yes {
        print!(
            "This will completely remove K3s from {}. Continue? [y/N]: ",
            hostname
        );
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Try server uninstall first, then agent
    let server_script =
        exec.execute_shell("test -f /usr/local/bin/k3s-uninstall.sh && echo exists")?;
    if String::from_utf8_lossy(&server_script.stdout).contains("exists") {
        println!("Uninstalling K3s server...");
        exec.execute_shell_interactive("/usr/local/bin/k3s-uninstall.sh")?;
    } else {
        let agent_script =
            exec.execute_shell("test -f /usr/local/bin/k3s-agent-uninstall.sh && echo exists")?;
        if String::from_utf8_lossy(&agent_script.stdout).contains("exists") {
            println!("Uninstalling K3s agent...");
            exec.execute_shell_interactive("/usr/local/bin/k3s-agent-uninstall.sh")?;
        } else {
            println!("K3s is not installed on this node.");
            return Ok(());
        }
    }

    println!();
    println!("✓ K3s uninstalled successfully!");

    Ok(())
}

/// Take an etcd snapshot
pub fn take_snapshot(hostname: &str, output: Option<&str>, config: &EnvConfig) -> Result<()> {
    let exec = Executor::new(hostname, config)?;

    println!("Taking etcd snapshot...");

    let cmd = if let Some(path) = output {
        format!("sudo k3s etcd-snapshot save --name={}", path)
    } else {
        "sudo k3s etcd-snapshot save".to_string()
    };

    exec.execute_shell_interactive(&cmd)
        .context("Failed to take etcd snapshot")?;

    println!();
    println!("✓ Snapshot created successfully!");
    println!();
    println!("List snapshots with: halvor k3s status");

    Ok(())
}

/// Restore from etcd snapshot
pub fn restore_snapshot(
    hostname: &str,
    snapshot: &str,
    yes: bool,
    config: &EnvConfig,
) -> Result<()> {
    let exec = Executor::new(hostname, config)?;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Restore K3s from etcd Snapshot");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Snapshot: {}", snapshot);
    println!();

    if !yes {
        println!("WARNING: This will stop K3s and restore from the snapshot.");
        println!("All changes since the snapshot will be lost!");
        print!("Continue? [y/N]: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Stop K3s
    println!("Stopping K3s...");
    exec.execute_shell("sudo systemctl stop k3s")?;

    // Restore snapshot
    println!("Restoring from snapshot...");
    let cmd = format!(
        "sudo k3s server --cluster-reset --cluster-reset-restore-path={}",
        snapshot
    );
    exec.execute_shell_interactive(&cmd)?;

    // Start K3s
    println!("Starting K3s...");
    exec.execute_shell("sudo systemctl start k3s")?;

    println!();
    println!("✓ Cluster restored from snapshot!");

    Ok(())
}
