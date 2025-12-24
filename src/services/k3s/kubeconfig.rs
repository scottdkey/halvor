//! K3s kubeconfig management

use crate::config::EnvConfig;
use crate::services::tailscale;
use crate::utils::exec::{CommandExecutor, Executor, local};
use anyhow::{Context, Result};

/// Fetch kubeconfig content from primary control plane node (without writing to file)
/// Returns the kubeconfig content as a string, with localhost/127.0.0.1 replaced with Tailscale address
pub fn fetch_kubeconfig_content(primary_hostname: &str, config: &EnvConfig) -> Result<String> {
    // Get kubeconfig from primary control plane node
    let primary_exec = Executor::new(primary_hostname, config)?;

    println!("  Fetching kubeconfig from {}...", primary_hostname);

    // First check if K3s service is running on the primary node (use same reliable method as init)
    let status_output = primary_exec
        .execute_simple("systemctl", &["is-active", "k3s"])
        .ok();

    let is_active = if let Some(output) = &status_output {
        let status_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let success = output.status.success();
        // systemctl is-active returns "active" if running, non-zero exit if not
        success && status_str == "active"
    } else {
        false
    };

    if !is_active {
        // Try with sudo in case systemctl requires it
        let sudo_status = primary_exec
            .execute_simple("sudo", &["systemctl", "is-active", "k3s"])
            .ok();
        let is_active_sudo = if let Some(output) = &sudo_status {
            let status_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let success = output.status.success();
            success && status_str == "active"
        } else {
            false
        };

        if !is_active_sudo {
            anyhow::bail!(
                "K3s service is not running on {}.\n\
                 Please ensure K3s is initialized and running:\n\
                 halvor k3s init -H {} -y\n\
                 Or check service status: ssh {} 'sudo systemctl status k3s'",
                primary_hostname,
                primary_hostname,
                primary_hostname
            );
        }
    }

    // Wait for kubeconfig to be available (with retries)
    // Use execute_shell to get kubeconfig directly without temp files
    // K3s with embedded etcd can take 1-2 minutes to fully initialize
    let kubeconfig_path = "/etc/rancher/k3s/k3s.yaml";
    let max_retries = 24; // 24 * 5 seconds = 2 minutes max (embedded etcd can be slow)
    let mut kubeconfig_content = None;

    for attempt in 1..=max_retries {
        // Try to get kubeconfig using k3s kubectl config view (more reliable than reading file)
        // This works even if the file doesn't exist yet, as long as k3s is running
        // First try reading the file directly (no sudo needed if readable)
        if let Ok(content) = primary_exec.read_file(kubeconfig_path) {
            if !content.trim().is_empty()
                && content.contains("apiVersion")
                && (content.contains("clusters") || content.contains("server"))
            {
                kubeconfig_content = Some(content);
                break;
            }
        }

        // If file read failed, try using k3s kubectl (requires sudo)
        // Use execute_interactive with sudo which handles password injection automatically
        // But we need to capture output, so use execute_shell with password injection
        // The execute_shell_interactive method handles sudo password injection
        let get_config_cmd = "sudo k3s kubectl config view --raw 2>&1";
        let get_config_output = primary_exec.execute_shell(get_config_cmd).ok();

        if let Some(output) = get_config_output {
            if output.status.success() {
                if let Ok(content) = String::from_utf8(output.stdout) {
                    // Check if content looks like valid kubeconfig (contains apiVersion)
                    if !content.trim().is_empty()
                        && !content.contains("Error")
                        && !content.contains("error")
                        && content.contains("apiVersion")
                        && (content.contains("clusters") || content.contains("server"))
                    {
                        kubeconfig_content = Some(content);
                        break;
                    }
                }
            }
        }

        if attempt < max_retries {
            println!(
                "  Kubeconfig not ready yet, waiting... (attempt {}/{})",
                attempt, max_retries
            );
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    }

    let mut kubeconfig_content = kubeconfig_content.ok_or_else(|| {
        anyhow::anyhow!(
            "Failed to read kubeconfig from {} after {} attempts. \
             K3s may still be initializing. Please wait a few minutes and try again, \
             or check K3s status: halvor k3s status -H {}",
            primary_hostname,
            max_retries,
            primary_hostname
        )
    })?;

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

    Ok(kubeconfig_content)
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
