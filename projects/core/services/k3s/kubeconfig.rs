//! K3s kubeconfig management

use crate::config::EnvConfig;
use crate::services::tailscale;
use crate::utils::exec::{CommandExecutor, Executor, local};
use anyhow::{Context, Result};

/// Fetch kubeconfig content from primary control plane node (without writing to file)
/// Returns the kubeconfig content as a string, with localhost/127.0.0.1 replaced with Tailscale address
pub fn fetch_kubeconfig_content(primary_hostname: &str, config: &EnvConfig) -> Result<String> {
    println!("  Fetching kubeconfig from {}...", primary_hostname);
    
    // Use Executor::new which will correctly detect if we're on the primary node
    // It checks local IPs and will return Executor::Local if we're already there
    let primary_exec = Executor::new(primary_hostname, config)
        .with_context(|| format!("Failed to create executor for primary node: {}", primary_hostname))?;
    
    // Check if we're on the primary node itself
    let is_local_primary = primary_exec.is_local();
    
    if is_local_primary {
        println!("  Primary node is local, using local execution");
    } else {
        println!("  Primary node is remote, connecting via SSH");
    }

    // First check if K3s service is running on the primary node
    // Use a direct check that reads output directly (more reliable over SSH)
    println!("  Checking K3s service status...");
    
    // Try without sudo first (systemctl is-active doesn't require sudo for status checks)
    let status_output = primary_exec
        .execute_shell("systemctl is-active k3s 2>/dev/null")
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
        println!("  Service check failed, trying with sudo (may prompt for password)...");
        let sudo_status = primary_exec
            .execute_shell("sudo systemctl is-active k3s 2>/dev/null")
            .ok();
        let is_active_sudo = if let Some(output) = &sudo_status {
            let status_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            // Check if output is "active" - this is the primary indicator
            // Don't rely on exit code as some systems may return non-zero even when active
            status_str == "active"
        } else {
            false
        };

        if !is_active_sudo {
            anyhow::bail!(
                "K3s service is not running on {}.\n\
                 Please ensure K3s is initialized and running:\n\
                 halvor init -H {} -y\n\
                 Or check service status: ssh {} 'sudo systemctl status k3s'",
                primary_hostname,
                primary_hostname,
                primary_hostname
            );
        } else {
            println!("  ✓ K3s service is running (checked with sudo)");
        }
    } else {
        println!("  ✓ K3s service is running");
    }

    // Get kubeconfig using k3s kubectl config view --raw
    // This is more reliable than reading the file directly, as it gets the config
    // from the running K3s service and doesn't require file permissions
    // Use tee to capture output while showing it in real-time
    println!("  Fetching kubeconfig via k3s kubectl (may prompt for sudo password)...");
    let kubeconfig_output_file = "/tmp/halvor_kubeconfig_output";
    let get_config_cmd = format!("sudo k3s kubectl config view --raw 2>&1 | tee {}", kubeconfig_output_file);
    
    let mut kubeconfig_content = None;
    
    // Try k3s kubectl config view first (most reliable)
    if let Ok(output) = primary_exec.execute_shell(&get_config_cmd) {
        if output.status.success() {
            // Read from the captured temp file using read_file (which handles piped output correctly)
            if let Ok(content) = primary_exec.read_file(kubeconfig_output_file) {
                if !content.trim().is_empty()
                    && content.contains("apiVersion")
                    && (content.contains("clusters") || content.contains("server"))
                {
                    println!("  ✓ Kubeconfig fetched via k3s kubectl");
                    kubeconfig_content = Some(content);
                }
            }
        }
    }
    
    // Fallback: try reading the file directly with sudo cat
    if kubeconfig_content.is_none() {
        println!("  Trying fallback: reading kubeconfig file directly (may prompt for sudo password)...");
        let kubeconfig_path = "/etc/rancher/k3s/k3s.yaml";
        let kubeconfig_file_output = "/tmp/halvor_kubeconfig_file_output";
        let sudo_cat_cmd = format!("sudo cat {} 2>&1 | tee {}", kubeconfig_path, kubeconfig_file_output);
        
        if let Ok(output) = primary_exec.execute_shell(&sudo_cat_cmd) {
            if output.status.success() {
                // Read from the captured temp file using read_file (which handles piped output correctly)
                if let Ok(content) = primary_exec.read_file(kubeconfig_file_output) {
                    if !content.trim().is_empty()
                        && content.contains("apiVersion")
                        && (content.contains("clusters") || content.contains("server"))
                    {
                        println!("  ✓ Kubeconfig fetched via file read");
                        kubeconfig_content = Some(content);
                    }
                }
            }
        }
    }
    
    let mut kubeconfig_content = kubeconfig_content.ok_or_else(|| {
        anyhow::anyhow!(
            "Failed to get kubeconfig from {}. Both 'k3s kubectl config view' and file read failed.\n\
             Ensure K3s is running and accessible.",
            primary_hostname
        )
    })?;
    
    // Validate that we got valid kubeconfig
    if kubeconfig_content.trim().is_empty() {
        anyhow::bail!("Kubeconfig file is empty");
    }
    if !kubeconfig_content.contains("apiVersion") {
        anyhow::bail!("Kubeconfig file does not contain apiVersion - file may be corrupted");
    }
    if !kubeconfig_content.contains("clusters") && !kubeconfig_content.contains("server") {
        anyhow::bail!("Kubeconfig file does not contain cluster configuration");
    }


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
