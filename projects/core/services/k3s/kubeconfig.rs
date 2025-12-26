//! K3s kubeconfig management

use crate::config::EnvConfig;
use crate::services::tailscale;
use crate::utils::exec::{CommandExecutor, Executor, local};
use anyhow::{Context, Result};
use yaml_rust::YamlLoader;

/// Fetch kubeconfig content from environment variable (1Password) or primary node
/// Returns the kubeconfig content as a string, with localhost/127.0.0.1 replaced with Tailscale address
pub fn fetch_kubeconfig_content(primary_hostname: &str, config: &EnvConfig) -> Result<String> {
    println!("  Fetching kubeconfig...");

    // Helper function to process kubeconfig content - replaces localhost with Tailscale address
    let process_kubeconfig = |mut kubeconfig: String, hostname: &str| -> Result<String> {
        // Get Tailscale IP/hostname for the node to replace 127.0.0.1
        let exec = Executor::new(hostname, config)
            .with_context(|| format!("Failed to create executor for node: {}", hostname))?;
        let tailscale_ip =
            tailscale::get_tailscale_ip_with_fallback(&exec, hostname, config)?;
        let tailscale_hostname = tailscale::get_tailscale_hostname_remote(&exec)
            .ok()
            .flatten();

        // Replace localhost/127.0.0.1 with Tailscale address
        let server_address = tailscale_hostname.as_ref().unwrap_or(&tailscale_ip);
        kubeconfig = kubeconfig.replace("127.0.0.1", server_address);
        kubeconfig = kubeconfig.replace("localhost", server_address);

        Ok(kubeconfig)
    };

    // First, try to read from local K3s installation if running on this machine
    if std::path::Path::new("/etc/rancher/k3s/k3s.yaml").exists() {
        println!("  ✓ Found local K3s installation");
        match local::execute_shell("sudo cat /etc/rancher/k3s/k3s.yaml") {
            Ok(output) if output.status.success() => {
                let mut kubeconfig = String::from_utf8_lossy(&output.stdout).to_string();
                println!("  ✓ Using kubeconfig from local K3s (/etc/rancher/k3s/k3s.yaml)");

                // Get local Tailscale IP/hostname without SSH
                let tailscale_status = local::execute_shell("tailscale status --json");
                if let Ok(status_output) = tailscale_status {
                    if status_output.status.success() {
                        let status_str = String::from_utf8_lossy(&status_output.stdout);
                        if let Ok(status_json) = serde_json::from_str::<serde_json::Value>(&status_str) {
                            // Get Tailscale hostname from Self field
                            if let Some(ts_hostname) = status_json.get("Self")
                                .and_then(|s| s.get("DNSName"))
                                .and_then(|d| d.as_str())
                                .map(|s| s.trim_end_matches('.').to_string()) {
                                println!("  Using Tailscale hostname: {}", ts_hostname);
                                kubeconfig = kubeconfig.replace("127.0.0.1", &ts_hostname);
                                kubeconfig = kubeconfig.replace("localhost", &ts_hostname);
                            } else if let Some(ts_ip) = status_json.get("Self")
                                .and_then(|s| s.get("TailscaleIPs"))
                                .and_then(|ips| ips.as_array())
                                .and_then(|arr| arr.get(0))
                                .and_then(|ip| ip.as_str()) {
                                println!("  Using Tailscale IP: {}", ts_ip);
                                kubeconfig = kubeconfig.replace("127.0.0.1", ts_ip);
                                kubeconfig = kubeconfig.replace("localhost", ts_ip);
                            }
                        }
                    }
                }

                return Ok(kubeconfig);
            }
            _ => {
                println!("  ⚠ Local K3s found but couldn't read kubeconfig (need sudo access)");
            }
        }
    }

    // Next, try to get from KUBE_CONFIG environment variable (from 1Password)
    if let Ok(kubeconfig_content) = std::env::var("KUBE_CONFIG") {
        println!("  ✓ Using kubeconfig from KUBE_CONFIG environment variable");
        return process_kubeconfig(kubeconfig_content, primary_hostname);
    }

    // If not found locally or in environment, provide helpful error message
    anyhow::bail!(
        "Kubeconfig not found. Tried:\n\
         1. Local K3s installation (/etc/rancher/k3s/k3s.yaml)\n\
         2. KUBE_CONFIG environment variable (from 1Password)\n\
         \n\
         Please either:\n\
         - Ensure K3s is installed locally, or\n\
         - Add the kubeconfig to your 1Password vault as KUBE_CONFIG=\"<content>\", or\n\
         - Specify a remote hostname with -H <hostname>"
    )
}

/// Set up kubeconfig on local machine from the primary control plane node
pub fn setup_local_kubeconfig(primary_hostname: &str, config: &EnvConfig) -> Result<()> {
    // Fetch kubeconfig content
    let kubeconfig_content = fetch_kubeconfig_content(primary_hostname, config)?;

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

/// Get kubeconfig from the cluster
#[allow(dead_code)]
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

/// Extract server URL and token from kubeconfig
/// Returns (server_hostname, token)
pub fn extract_server_and_token_from_kubeconfig(kubeconfig_content: &str) -> Result<(String, String)> {
    // Clean up the kubeconfig content - remove trailing '=' characters that 1Password might add
    // These appear on multi-line values and break YAML parsing
    let cleaned_content = kubeconfig_content
        .lines()
        .map(|line| line.trim_end_matches('='))
        .collect::<Vec<_>>()
        .join("\n");

    // Parse the kubeconfig YAML
    let docs = YamlLoader::load_from_str(&cleaned_content)
        .context("Failed to parse kubeconfig YAML")?;

    let doc = docs
        .first()
        .ok_or_else(|| anyhow::anyhow!("Empty kubeconfig"))?;

    // Extract server URL from clusters[0].cluster.server
    let server = doc["clusters"][0]["cluster"]["server"]
        .as_str()
        .ok_or_else(|| {
            // Provide helpful debugging info
            let clusters = &doc["clusters"];
            anyhow::anyhow!(
                "No server found in kubeconfig clusters. Clusters structure: {:?}",
                clusters
            )
        })?;

    // Extract token from users[0].user.token
    let token = doc["users"][0]["user"]["token"]
        .as_str()
        .or_else(|| {
            // Also try client-certificate-data if token is not present
            doc["users"][0]["user"]["client-certificate-data"].as_str()
        })
        .ok_or_else(|| anyhow::anyhow!("No token or client-certificate-data found in kubeconfig users"))?
        .to_string();

    // Parse server URL to extract hostname (remove https:// and port)
    let server_host = server
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split(':')
        .next()
        .ok_or_else(|| anyhow::anyhow!("Invalid server URL in kubeconfig"))?
        .to_string();

    Ok((server_host, token))
}
