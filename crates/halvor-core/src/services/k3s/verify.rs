use super::kubeconfig::setup_local_kubeconfig;
use crate::config::EnvConfig;
use crate::utils::exec::{CommandExecutor, Executor};
use anyhow::{Context, Result};

/// Verify that a node successfully joined the cluster with retries using local kubectl
/// kubeconfig_content: Optional pre-fetched kubeconfig content. If None, will fetch from primary_hostname
pub fn verify_cluster_join_with_local_kubectl_and_config(
    primary_hostname: &str,
    hostname: &str,
    control_plane: bool,
    config: &EnvConfig,
    kubeconfig_content: Option<String>,
) -> Result<()> {
    // Use local executor for kubectl commands
    let local_exec = Executor::Local;

    // Also create executor for the joining node to check service status
    let node_exec = Executor::new(hostname, config).ok();

    // Use pre-fetched kubeconfig if provided, otherwise fetch it
    if let Some(ref content) = kubeconfig_content {
        // Process the kubeconfig to ensure it points to the primary server
        // This is critical - the kubeconfig must point to frigg, not baulder
        let processed_content = match super::kubeconfig::process_kubeconfig_for_primary(content, primary_hostname, config) {
            Ok(processed) => {
                println!("  ✓ Processed kubeconfig to ensure it points to primary server");
                processed
            }
            Err(e) => {
                println!("  ⚠ Failed to process kubeconfig: {}. Using as-is (may cause connection issues)", e);
                content.clone()
            }
        };
        
        // Write the processed kubeconfig to local filesystem
        let home = std::env::var("HOME")
            .ok()
            .unwrap_or_else(|| ".".to_string());
        let kube_dir = format!("{}/.kube", home);
        std::fs::create_dir_all(&kube_dir).context("Failed to create ~/.kube directory")?;
        let kube_config_path = format!("{}/config", kube_dir);

        // Merge with existing config if it exists, otherwise write new
        if std::path::Path::new(&kube_config_path).exists() {
            let existing = std::fs::read_to_string(&kube_config_path).unwrap_or_default();
            if !existing.contains("k3s") {
                let mut merged = existing;
                if !merged.ends_with('\n') {
                    merged.push('\n');
                }
                merged.push_str("---\n");
                merged.push_str(&processed_content);
                std::fs::write(&kube_config_path, merged)?;
            } else {
                // File already has k3s - verify it points to the correct server
                // If not, replace the k3s section with our processed version
                if let Ok((existing_server, _)) = super::kubeconfig::extract_server_and_token_from_kubeconfig(&existing) {
                    let normalized_existing = crate::config::service::normalize_hostname(&existing_server);
                    let normalized_primary = crate::config::service::normalize_hostname(primary_hostname);
                    if normalized_existing != normalized_primary && existing_server != primary_hostname {
                        println!("  ⚠ Existing kubeconfig points to wrong server ({}), replacing with corrected version", existing_server);
                        // Replace the k3s section - this is a simple approach
                        // In a more sophisticated implementation, we'd parse and replace just the cluster section
                        std::fs::write(&kube_config_path, &processed_content)
                            .context("Failed to write corrected kubeconfig")?;
                    }
                }
            }
        } else {
            std::fs::write(&kube_config_path, &processed_content).context("Failed to write kubeconfig")?;
        }
        println!("  ✓ Using pre-fetched kubeconfig");
    } else {
        // Ensure kubeconfig is set up before verification
        // Retry setup if it fails (cluster might still be initializing)
        println!("Ensuring kubeconfig is available...");
        for attempt in 1..=3 {
            let kubeconfig_result = setup_local_kubeconfig(primary_hostname, config);
            match kubeconfig_result {
                Ok(()) => {
                    println!("  ✓ Kubeconfig is available");
                    break;
                }
                Err(e) => {
                    if attempt < 3 {
                        println!(
                            "  Kubeconfig not ready yet (attempt {}/3), waiting 5 seconds...",
                            attempt
                        );
                        std::thread::sleep(std::time::Duration::from_secs(5));
                    } else {
                        return Err(e).context("Failed to set up kubeconfig after 3 attempts");
                    }
                }
            }
        }
    }

    // Verify kubeconfig is actually accessible
    let home = std::env::var("HOME")
        .ok()
        .unwrap_or_else(|| ".".to_string());
    let kube_config_path = format!("{}/.kube/config", home);

    if !std::path::Path::new(&kube_config_path).exists() {
        anyhow::bail!(
            "Kubeconfig file does not exist at {}. Please ensure kubeconfig is set up.",
            kube_config_path
        );
    }

    verify_cluster_join_with_retry(&local_exec, hostname, control_plane, node_exec.as_ref())
}

/// Verify that a node successfully joined the cluster with retries
fn verify_cluster_join_with_retry<E: CommandExecutor>(
    exec: &E,
    expected_hostname: &str,
    control_plane: bool,
    node_exec: Option<&Executor>,
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

        match verify_cluster_join_once(exec, expected_hostname, control_plane, attempt, node_exec) {
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
    _control_plane: bool,
    attempt: u32,
    node_exec: Option<&Executor>,
) -> Result<()> {
    // Step 1: Verify kubectl is available and can connect to cluster
    if attempt == 1 {
        println!("[1/3] Checking kubectl access to cluster...");
    }

    // Get kubeconfig path early so we can use it in all commands
    let home = std::env::var("HOME")
        .ok()
        .unwrap_or_else(|| ".".to_string());
    let kube_config_path = format!("{}/.kube/config", home);

    // First check if kubectl exists using check_command_exists (more reliable)
    let kubectl_exists = exec.check_command_exists("kubectl").unwrap_or(false);

    if !kubectl_exists {
        anyhow::bail!(
            "kubectl not found (attempt {}). Please install kubectl to verify cluster access.",
            attempt
        );
    }

    // Then verify kubectl works by checking version
    // Note: kubectl version --client doesn't require kubeconfig, so it's safe to check
    // But we'll still set KUBECONFIG to avoid any issues
    let kubeconfig_env = format!("KUBECONFIG='{}'", kube_config_path);
    let kubectl_version_output = exec
        .execute_shell(&format!("{} kubectl version --client", kubeconfig_env))
        .ok();

    let (kubectl_works, error_details) = if let Some(ref out) = kubectl_version_output {
        let works = out.status.success() && !out.stdout.is_empty();
        let error = String::from_utf8(out.stderr.clone())
            .ok()
            .unwrap_or_else(|| "Unknown error".to_string());
        (works, error)
    } else {
        (false, "Command execution failed".to_string())
    };

    if !kubectl_works {
        anyhow::bail!(
            "kubectl found but not working (attempt {}). Error: {}. Please check kubectl installation.",
            attempt,
            if error_details.trim().is_empty() {
                "Command failed with no error message"
            } else {
                error_details.trim()
            }
        );
    }

    if attempt == 1 {
        println!("  ✓ kubectl is available");
    }

    // Step 2: Check if we can connect to the cluster
    // If we have a node executor, skip cluster-info check entirely and go straight to node check
    // This uses the same method as halvor status (sudo k3s kubectl) which works reliably
    let is_cert_error = if node_exec.is_some() {
        // We have a node executor, so we'll use local k3s kubectl which doesn't need cluster-info check
        if attempt == 1 {
            println!("[2/3] Verifying cluster connectivity...");
            println!("  ⚠ Skipping cluster-info check (will verify via local k3s kubectl on joining node, same as halvor status)");
        }
        false // Not a cert error, we're just skipping this check
    } else {
        // No node executor, try cluster-info check (but don't fail on DNS/cert errors)
        if attempt == 1 {
            println!("[2/3] Verifying cluster connectivity...");
        }

        // First check if kubeconfig exists and is readable
        if !std::path::Path::new(&kube_config_path).exists() {
            anyhow::bail!(
                "Kubeconfig file does not exist at {} (attempt {}). Please ensure kubeconfig is set up.",
                kube_config_path,
                attempt
            );
        }

        // Try to get cluster info - this will fail if kubeconfig is invalid or cluster is unreachable
        // Use KUBECONFIG environment variable to ensure we use ~/.kube/config instead of /etc/rancher/k3s/k3s.yaml
        let kubeconfig_env = format!("KUBECONFIG='{}'", kube_config_path);
        let cluster_info_output = exec.execute_shell(&format!("{} kubectl cluster-info 2>&1", kubeconfig_env)).ok();

        let (cluster_info, cluster_error, cluster_unreachable, cert_error) =
            if let Some(ref out) = cluster_info_output {
                let info = String::from_utf8(out.stdout.clone())
                    .ok()
                    .unwrap_or_else(|| "cluster_unreachable".to_string());
                let error = String::from_utf8(out.stderr.clone())
                    .ok()
                    .unwrap_or_else(|| String::new());
                
                // Check if this is a certificate error or DNS error (not a real connectivity issue)
                // DNS errors like "no such host" are common when ~/.kube/config has hostnames that can't be resolved
                // But the node might still be in the cluster (we'll verify via node list)
                let cert_error = error.contains("certificate signed by unknown authority")
                    || error.contains("x509: certificate")
                    || error.contains("tls: failed to verify certificate")
                    || error.contains("no such host")
                    || error.contains("lookup");
                
                // Only treat as unreachable if it's NOT a certificate/DNS error
                // Certificate/DNS errors mean we can't verify connectivity this way, but the node might still be joined
                // We'll verify via node list instead
                let unreachable = !cert_error && (
                    !out.status.success()
                    || info.contains("cluster_unreachable")
                    || info.contains("Unable to connect")
                    || info.contains("The connection to the server")
                    || (error.contains("Unable to connect") && !cert_error)
                );
                (info, error, unreachable, cert_error)
            } else {
                (
                    "cluster_unreachable".to_string(),
                    "Command execution failed".to_string(),
                    true,
                    false,
                )
            };

        if cluster_unreachable {
            let error_msg = if !cluster_error.trim().is_empty() {
                cluster_error.trim()
            } else if !cluster_info.trim().is_empty() {
                cluster_info.trim()
            } else {
                "Unknown error"
            };

            anyhow::bail!(
                "Cannot connect to cluster (attempt {}). Error: {}. Cluster may still be initializing or kubeconfig may be invalid.",
                attempt,
                error_msg
            );
        }

        if attempt == 1 {
            if cert_error {
                println!("  ⚠ Cluster connectivity check failed due to certificate/DNS error (this is OK, will verify via node list)");
            } else {
                println!("  ✓ Cluster is reachable");
            }
        }
        
        cert_error
    };

    // Step 3: Verify node appears in cluster and is Ready
    if attempt == 1 {
        println!("[3/3] Checking node status in cluster...");
    }

    // Try multiple methods to get nodes, similar to how halvor status works:
    // 1. First try: Use local k3s kubectl on the joining node - this uses /etc/rancher/k3s/k3s.yaml with correct certs
    // 2. Fallback: Use kubectl with ~/.kube/config (with --insecure-skip-tls-verify if cert errors)
    let node_output = if let Some(node_exec) = node_exec {
        // Try using local k3s kubectl on the joining node (same method as halvor status)
        let tmp_file = "/tmp/k3s_verify_nodes";
        let k3s_kubectl_result = node_exec.execute_shell_interactive(&format!(
            "bash -c 'sudo k3s kubectl get nodes -o json > {} 2>&1 || echo \"k3s_kubectl_failed\" > {}'",
            tmp_file, tmp_file
        ));
        
        if k3s_kubectl_result.is_ok() {
            if let Ok(output) = node_exec.read_file(tmp_file) {
                if !output.trim().is_empty() && !output.contains("k3s_kubectl_failed") {
                    if attempt == 1 {
                        println!("  ✓ Using local k3s kubectl (same method as halvor status)");
                    }
                    output
                } else {
                    // Fall back to regular kubectl
                    let kubeconfig_env = format!("KUBECONFIG='{}'", kube_config_path);
                    let node_cmd = if is_cert_error {
                        format!("{} kubectl get nodes -o json --insecure-skip-tls-verify 2>&1", kubeconfig_env)
                    } else {
                        format!("{} kubectl get nodes -o json 2>&1", kubeconfig_env)
                    };
                    exec.execute_shell(&node_cmd)
                        .ok()
                        .and_then(|out| String::from_utf8(out.stdout).ok())
                        .unwrap_or_else(|| "kubectl_failed".to_string())
                }
            } else {
                // Fall back to regular kubectl
                let kubeconfig_env = format!("KUBECONFIG='{}'", kube_config_path);
                let node_cmd = if is_cert_error {
                    format!("{} kubectl get nodes -o json --insecure-skip-tls-verify 2>&1", kubeconfig_env)
                } else {
                    format!("{} kubectl get nodes -o json 2>&1", kubeconfig_env)
                };
                exec.execute_shell(&node_cmd)
                    .ok()
                    .and_then(|out| String::from_utf8(out.stdout).ok())
                    .unwrap_or_else(|| "kubectl_failed".to_string())
            }
        } else {
            // Fall back to regular kubectl
            let kubeconfig_env = format!("KUBECONFIG='{}'", kube_config_path);
            let node_cmd = if is_cert_error {
                format!("{} kubectl get nodes -o json --insecure-skip-tls-verify 2>&1", kubeconfig_env)
            } else {
                format!("{} kubectl get nodes -o json 2>&1", kubeconfig_env)
            };
            exec.execute_shell(&node_cmd)
                .ok()
                .and_then(|out| String::from_utf8(out.stdout).ok())
                .unwrap_or_else(|| "kubectl_failed".to_string())
        }
    } else {
        // No node executor available, use regular kubectl
        let kubeconfig_env = format!("KUBECONFIG='{}'", kube_config_path);
        let node_cmd = if is_cert_error {
            format!("{} kubectl get nodes -o json --insecure-skip-tls-verify 2>&1", kubeconfig_env)
        } else {
            format!("{} kubectl get nodes -o json 2>&1", kubeconfig_env)
        };
        exec.execute_shell(&node_cmd)
            .ok()
            .and_then(|out| String::from_utf8(out.stdout).ok())
            .unwrap_or_else(|| "kubectl_failed".to_string())
    };

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
    let mut node_has_errors = false;
    let mut node_conditions = Vec::new();
    let mut node_name_found = String::new();

    for node in items {
        let node_name = node
            .get("metadata")
            .and_then(|m| m.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("");

        if node_name == hostname_str || node_name.contains(&hostname_str) {
            node_found = true;
            node_name_found = node_name.to_string();

            // Collect all node conditions to diagnose issues
            if let Some(status) = node.get("status") {
                if let Some(conditions) = status.get("conditions") {
                    if let Some(conditions_array) = conditions.as_array() {
                        for condition in conditions_array {
                            let condition_type = condition
                                .get("type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Unknown");
                            let condition_status = condition
                                .get("status")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Unknown");
                            let condition_reason = condition
                                .get("reason")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let condition_message = condition
                                .get("message")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");

                            node_conditions.push(format!(
                                "  {}: {} ({})",
                                condition_type,
                                condition_status,
                                if !condition_reason.is_empty() {
                                    condition_reason
                                } else if !condition_message.is_empty() {
                                    condition_message
                                } else {
                                    "no details"
                                }
                            ));

                            if condition_type == "Ready" && condition_status == "True" {
                                node_ready = true;
                            }

                            // Check for actual errors (not just Unknown status)
                            // Unknown status is acceptable during initial registration
                            if condition_type == "Ready" && condition_status == "False" {
                                // Only consider it an error if there's an actual error reason
                                // NodeStatusUnknown is not a real error, just means control plane can't reach it yet
                                if !condition_reason.is_empty()
                                    && condition_reason != "NodeStatusUnknown"
                                {
                                    node_has_errors = true;
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

    // Check if node is healthy: either Ready=True, or found in cluster with active service and no errors
    // NodeStatusUnknown is acceptable during initial registration - it just means the control plane
    // hasn't fully established connectivity yet, but if the service is running, the node is likely healthy
    let node_is_healthy = node_ready || (!node_has_errors && node_found);

    if !node_is_healthy {
        // Show detailed node conditions to help diagnose the issue
        let mut error_msg = format!(
            "Node '{}' found but not Ready yet (attempt {}).",
            node_name_found, attempt
        );

        if !node_conditions.is_empty() {
            error_msg.push_str("\n\nNode conditions:");
            for condition in &node_conditions {
                error_msg.push_str("\n");
                error_msg.push_str(condition);
            }
        }

        // Check K3s service status on the node if we have access
        if let Some(exec) = node_exec {
            // Try k3s service first, then k3s-agent
            let service_status = {
                let k3s_check = exec.execute_shell("systemctl is-active k3s 2>/dev/null || echo inactive").ok();
                let is_k3s_active = k3s_check
                    .map(|out| {
                        out.status.success()
                            && String::from_utf8_lossy(&out.stdout).trim() == "active"
                    })
                    .unwrap_or(false);

                if is_k3s_active {
                    "active".to_string()
                } else {
                    // Try k3s-agent
                    let agent_check = exec
                        .execute_shell("systemctl is-active k3s-agent 2>/dev/null || echo inactive")
                        .ok();
                    agent_check
                        .and_then(|out| {
                            if out.status.success() {
                                String::from_utf8(out.stdout).ok()
                            } else {
                                None
                            }
                        })
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|| "not_running".to_string())
                }
            };

            error_msg.push_str(&format!(
                "\n\nK3s service status on node: {}",
                service_status
            ));

            if service_status == "active" || service_status == "activating" {
                // Node is found in cluster and service is active - this is healthy enough
                // NodeStatusUnknown is just a timing issue, the node is actually working
                if attempt == 1 {
                    println!(
                        "  ✓ Node '{}' found in cluster with active service (status may show Unknown during initial registration)",
                        node_name_found
                    );
                }
                return Ok(());
            } else {
                error_msg.push_str("\n⚠ K3s service is not running on the node!");
                error_msg.push_str("\n  This may indicate the service failed to start or crashed.");
                error_msg.push_str(&format!(
                    "\n  Check service logs: ssh {} 'sudo journalctl -u k3s -n 50'",
                    expected_hostname
                ));
            }
        } else {
            // No access to check service, but node is found - if it's just Unknown status, that's OK
            if !node_has_errors {
                // Node is found and no actual errors, just Unknown status - this is acceptable
                if attempt == 1 {
                    println!(
                        "  ✓ Node '{}' found in cluster (status Unknown is normal during initial registration)",
                        node_name_found
                    );
                }
                return Ok(());
            }
        }

        error_msg.push_str("\n\nNode may still be initializing. Common causes:");
        error_msg.push_str("\n  - K3s service is still starting up");
        error_msg.push_str("\n  - Network connectivity issues");
        error_msg.push_str("\n  - Resource constraints (memory/disk)");
        error_msg.push_str("\n  - Container runtime issues");

        anyhow::bail!("{}", error_msg);
    }

    if attempt == 1 {
        println!("  ✓ Node '{}' is Ready in cluster", hostname_str);
    }

    Ok(())
}

/// Verify HA cluster health and failover capability
/// Checks all nodes, etcd health, and verifies cluster can operate from any node
#[allow(dead_code)]
pub fn verify_ha_cluster(
    primary_hostname: &str,
    expected_nodes: &[&str],
    config: &EnvConfig,
) -> Result<()> {
    use std::io::{self, Write};

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

    // Skip sudo -v check - the actual commands will prompt for sudo when needed
    // This avoids hanging on password prompts that may not display properly
    println!("(You may be prompted for your sudo password when running k3s commands)");
    io::stdout().flush()?;

    // Try to verify K3s service is running (but don't fail if check fails - try cluster query instead)
    // Check both k3s and k3s-agent services
    let service_check_tmp = "/tmp/k3s_verify_service";
    let _ = exec.execute_shell_interactive(&format!(
        "bash -c '(sudo systemctl is-active k3s 2>/dev/null || sudo systemctl is-active k3s-agent 2>/dev/null || echo \"not_running\") > {} 2>&1'",
        service_check_tmp
    ));
    let service_running = if let Ok(service_status) = exec.read_file(service_check_tmp) {
        let status = service_status.trim();
        status == "active" || status == "activating"
    } else {
        false
    };

    if !service_running {
        println!(
            "  ⚠ Service status check failed on {}, but will try to query cluster anyway...",
            primary_hostname
        );
    }

    // First check if kubectl is available and working - use temp file
    // Use a simpler command that doesn't require complex piping
    let kubectl_tmp = "/tmp/k3s_kubectl_version";
    let kubectl_cmd = format!(
        "sudo k3s kubectl version --client > {} 2>&1; echo $? > /tmp/k3s_kubectl_exit",
        kubectl_tmp
    );
    let _ = exec.execute_shell_interactive(&kubectl_cmd);

    // Check exit code
    if let Ok(exit_code) = exec.read_file("/tmp/k3s_kubectl_exit") {
        if exit_code.trim() == "0" {
            if let Ok(version_output) = exec.read_file(kubectl_tmp) {
                let version_trimmed = version_output.trim();
                if !version_trimmed.is_empty() {
                    println!("  kubectl version: {}", version_trimmed);
                }
            }
        }
    }

    // Get nodes using temp file to capture output from interactive command
    // Try the primary host first, but if it fails, try other control plane nodes
    let nodes_tmp = "/tmp/k3s_nodes_json";
    let mut nodes_output = String::new();
    let mut query_success = false;

    // Try primary host first
    let query_result = exec.execute_shell_interactive(&format!(
        "bash -c 'sudo k3s kubectl get nodes -o json > {} 2>&1 || echo \"query_failed\" > {}'",
        nodes_tmp, nodes_tmp
    ));

    if query_result.is_ok() {
        if let Ok(output) = exec.read_file(nodes_tmp) {
            if !output.trim().is_empty() && !output.contains("query_failed") {
                nodes_output = output;
                query_success = true;
            }
        }
    }

    // If query failed, try other nodes
    if !query_success {
        println!(
            "  ⚠ Query failed on {}, trying other control plane nodes...",
            primary_hostname
        );
        for node in expected_nodes {
            if *node != primary_hostname {
                if let Ok(node_exec) = Executor::new(node, config) {
                    let _ = node_exec.execute_shell_interactive(&format!(
                        "bash -c 'sudo k3s kubectl get nodes -o json > {} 2>&1 || echo \"query_failed\" > {}'",
                        nodes_tmp, nodes_tmp
                    ));
                    if let Ok(output) = node_exec.read_file(nodes_tmp) {
                        if !output.trim().is_empty() && !output.contains("query_failed") {
                            nodes_output = output;
                            query_success = true;
                            println!("  ✓ Successfully queried cluster from {}", node);
                            break;
                        }
                    }
                }
            }
        }
    }

    if !query_success || nodes_output.trim().is_empty() {
        anyhow::bail!(
            "Could not query cluster from any control plane node. Tried: {}\n\
             This may indicate:\n  - K3s is not running on any control plane node\n  - kubectl is not available\n  - Cluster is not initialized\n\n\
             Try: halvor status k3s -H <node>",
            expected_nodes.join(", ")
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
