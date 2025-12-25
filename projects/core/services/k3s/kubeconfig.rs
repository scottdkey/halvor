//! K3s kubeconfig management

use crate::config::EnvConfig;
use crate::services::tailscale;
use crate::utils::exec::{CommandExecutor, Executor, local};
use anyhow::{Context, Result};

/// Fetch kubeconfig content from environment variable (1Password) or primary node
/// Returns the kubeconfig content as a string, with localhost/127.0.0.1 replaced with Tailscale address
pub fn fetch_kubeconfig_content(primary_hostname: &str, config: &EnvConfig) -> Result<String> {
    println!("  Fetching kubeconfig...");
    
    // Helper function to process kubeconfig content
    let process_kubeconfig = |mut kubeconfig: String| -> Result<String> {
        // Get Tailscale IP/hostname for the primary node to replace 127.0.0.1
        let primary_exec = Executor::new(primary_hostname, config)
            .with_context(|| format!("Failed to create executor for primary node: {}", primary_hostname))?;
        let tailscale_ip =
            tailscale::get_tailscale_ip_with_fallback(&primary_exec, primary_hostname, config)?;
        let tailscale_hostname = tailscale::get_tailscale_hostname_remote(&primary_exec)
            .ok()
            .flatten();

        // Replace localhost/127.0.0.1 with Tailscale address
        let server_address = tailscale_hostname.as_ref().unwrap_or(&tailscale_ip);
        kubeconfig = kubeconfig.replace("127.0.0.1", server_address);
        kubeconfig = kubeconfig.replace("localhost", server_address);
        
        Ok(kubeconfig)
    };
    
    // Get kubeconfig from KUBE_CONFIG environment variable (from 1Password)
    if let Ok(kubeconfig_content) = std::env::var("KUBE_CONFIG") {
        println!("  ✓ Using kubeconfig from KUBE_CONFIG environment variable");
        return process_kubeconfig(kubeconfig_content);
    }
    
    // If not in environment variable, provide helpful error message
    anyhow::bail!(
        "Kubeconfig not found in KUBE_CONFIG environment variable.\n\
         Please add the kubeconfig to your 1Password vault and set it as:\n\
         KUBE_CONFIG=\"<kubeconfig-content>\"\n\
         \n\
         The kubeconfig should have been printed during cluster initialization.\n\
         Copy it from the init output and add it to your 1Password environment variables."
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
