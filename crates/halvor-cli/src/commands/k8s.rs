//! Kubernetes context management for halvor
//!
//! This module provides commands to switch between different kubeconfig contexts:
//! - `halvor` (direct): Direct connection to K3s API server
//! - `tailscale-operator`: Connection via Tailscale operator (works from anywhere on tailnet)

use anyhow::{Context, Result};
use std::process::Command;

#[derive(clap::Subcommand, Clone, Debug)]
pub enum K8sCommands {
    /// Switch to direct K3s context (halvor)
    #[command(name = "direct", alias = "local")]
    Direct,
    /// Switch to Tailscale operator context
    #[command(name = "tailscale", alias = "ts")]
    Tailscale,
    /// Show current context and list available contexts
    #[command(name = "context", alias = "ctx")]
    Context,
    /// Set up both contexts (direct and tailscale)
    Setup {
        /// Primary control plane hostname for direct context (defaults to frigg)
        #[arg(long, default_value = "frigg")]
        server: String,
    },
}

/// Handle k8s subcommands
pub fn handle_k8s(command: K8sCommands) -> Result<()> {
    match command {
        K8sCommands::Direct => switch_to_direct_context(),
        K8sCommands::Tailscale => switch_to_tailscale_context(),
        K8sCommands::Context => show_contexts(),
        K8sCommands::Setup { server } => setup_contexts(&server),
    }
}

/// Switch to the direct K3s context
fn switch_to_direct_context() -> Result<()> {
    // First check if halvor context exists
    let contexts = get_available_contexts()?;

    // Look for a direct context - could be named "halvor", "default", or the cluster name
    let direct_context = contexts.iter().find(|c| {
        *c == "halvor" || *c == "default" || c.contains("k3s") || c.contains("frigg")
    });

    if let Some(ctx) = direct_context {
        switch_context(ctx)?;
        println!("✓ Switched to direct context: {}", ctx);
        println!("  This connects directly to the K3s API server");
    } else {
        println!("No direct K3s context found.");
        println!("\nAvailable contexts:");
        for ctx in &contexts {
            println!("  - {}", ctx);
        }
        println!("\nTo set up the direct context, run:");
        println!("  halvor k8s setup --server=frigg");
        println!("\nOr manually fetch kubeconfig from your control plane:");
        println!("  scp frigg:/etc/rancher/k3s/k3s.yaml ~/.kube/config");
    }

    Ok(())
}

/// Switch to the Tailscale operator context
fn switch_to_tailscale_context() -> Result<()> {
    let contexts = get_available_contexts()?;

    // Look for tailscale context
    let ts_context = contexts.iter().find(|c| {
        c.contains("tailscale") || c.contains("ts.net")
    });

    if let Some(ctx) = ts_context {
        switch_context(ctx)?;
        println!("✓ Switched to Tailscale operator context: {}", ctx);
        println!("  This connects via the Tailscale Kubernetes operator");
    } else {
        // Try to configure it
        println!("Tailscale operator context not found. Attempting to configure...");

        let output = Command::new("tailscale")
            .args(["configure", "kubeconfig", "tailscale-operator"])
            .output()
            .context("Failed to run tailscale configure kubeconfig")?;

        if output.status.success() {
            println!("✓ Tailscale operator context configured");

            // Now try to switch to it
            let new_contexts = get_available_contexts()?;
            if let Some(ctx) = new_contexts.iter().find(|c| c.contains("tailscale") || c.contains("ts.net")) {
                switch_context(ctx)?;
                println!("✓ Switched to Tailscale operator context: {}", ctx);
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("Failed to configure Tailscale operator context:");
            println!("  {}", stderr);
            println!("\nMake sure the Tailscale Kubernetes operator is installed in your cluster:");
            println!("  helm upgrade --install tailscale-operator tailscale/tailscale-operator \\");
            println!("    --namespace=tailscale --create-namespace \\");
            println!("    --set-string oauth.clientId=\"<ID>\" \\");
            println!("    --set-string oauth.clientSecret=\"<SECRET>\" \\");
            println!("    --set-string apiServerProxyConfig.mode=\"true\"");
        }
    }

    Ok(())
}

/// Show current context and list all available contexts
fn show_contexts() -> Result<()> {
    // Get current context
    let current = Command::new("kubectl")
        .args(["config", "current-context"])
        .output()
        .context("Failed to get current kubectl context")?;

    let current_ctx = if current.status.success() {
        String::from_utf8_lossy(&current.stdout).trim().to_string()
    } else {
        "(none)".to_string()
    };

    println!("Current context: {}", current_ctx);
    println!();

    // Get all contexts
    let contexts = get_available_contexts()?;

    println!("Available contexts:");
    for ctx in &contexts {
        let marker = if ctx == &current_ctx { " *" } else { "" };
        let ctx_type = if ctx.contains("tailscale") || ctx.contains("ts.net") {
            "(tailscale)"
        } else if ctx == "halvor" || ctx == "default" || ctx.contains("k3s") {
            "(direct)"
        } else {
            ""
        };
        println!("  {} {}{}", ctx, ctx_type, marker);
    }

    println!();
    println!("Switch contexts:");
    println!("  halvor k8s direct    - Use direct K3s connection");
    println!("  halvor k8s tailscale - Use Tailscale operator");

    Ok(())
}

/// Set up both direct and Tailscale contexts
fn setup_contexts(server: &str) -> Result<()> {
    println!("Setting up Kubernetes contexts...");
    println!();

    // 1. Set up direct context by fetching kubeconfig from server
    println!("1. Setting up direct context from {}...", server);

    let halvor_dir = halvor_core::config::find_halvor_dir()?;
    let config = halvor_core::config::load_env_config(&halvor_dir)?;

    // Get the server's Tailscale hostname or IP
    let server_addr = if let Some(host_config) = config.hosts.get(server) {
        host_config.hostname.clone()
            .or(host_config.ip.clone())
            .unwrap_or_else(|| server.to_string())
    } else {
        server.to_string()
    };

    // Create ~/.kube directory if it doesn't exist
    let home = std::env::var("HOME").context("HOME environment variable not set")?;
    let kube_dir = std::path::PathBuf::from(home).join(".kube");
    std::fs::create_dir_all(&kube_dir)?;

    // Fetch kubeconfig from server
    let kubeconfig_path = kube_dir.join("config");
    let halvor_kubeconfig_path = kube_dir.join("halvor-direct.yaml");

    println!("   Fetching kubeconfig from {}...", server_addr);
    let scp_output = Command::new("scp")
        .args([
            &format!("{}:/etc/rancher/k3s/k3s.yaml", server_addr),
            halvor_kubeconfig_path.to_str().unwrap(),
        ])
        .output()
        .context("Failed to fetch kubeconfig via scp")?;

    if scp_output.status.success() {
        // Update the server address in the fetched kubeconfig
        let kubeconfig_content = std::fs::read_to_string(&halvor_kubeconfig_path)?;
        let updated_content = kubeconfig_content
            .replace("127.0.0.1", &server_addr)
            .replace("localhost", &server_addr);
        std::fs::write(&halvor_kubeconfig_path, updated_content)?;

        println!("   ✓ Direct kubeconfig saved to {}", halvor_kubeconfig_path.display());

        // Merge into main kubeconfig or set as KUBECONFIG
        println!("   Merging into kubectl config...");

        // Use kubectl to merge configs
        let merge_output = Command::new("sh")
            .args([
                "-c",
                &format!(
                    "KUBECONFIG={}:{} kubectl config view --flatten > {}.tmp && mv {}.tmp {}",
                    kubeconfig_path.display(),
                    halvor_kubeconfig_path.display(),
                    kubeconfig_path.display(),
                    kubeconfig_path.display(),
                    kubeconfig_path.display()
                ),
            ])
            .output();

        if merge_output.is_ok() && merge_output.unwrap().status.success() {
            println!("   ✓ Merged into kubectl config");
        } else {
            println!("   ⚠ Could not merge. Set KUBECONFIG manually:");
            println!("     export KUBECONFIG={}", halvor_kubeconfig_path.display());
        }
    } else {
        let stderr = String::from_utf8_lossy(&scp_output.stderr);
        println!("   ⚠ Could not fetch kubeconfig: {}", stderr.trim());
        println!("   Make sure you can SSH to {}", server_addr);
    }

    println!();

    // 2. Set up Tailscale operator context
    println!("2. Setting up Tailscale operator context...");

    let ts_output = Command::new("tailscale")
        .args(["configure", "kubeconfig", "tailscale-operator"])
        .output()
        .context("Failed to run tailscale configure kubeconfig")?;

    if ts_output.status.success() {
        println!("   ✓ Tailscale operator context configured");
    } else {
        let stderr = String::from_utf8_lossy(&ts_output.stderr);
        if stderr.contains("not found") || stderr.contains("no such") {
            println!("   ⚠ Tailscale operator not found in tailnet");
            println!("   Make sure the operator is installed in your cluster");
        } else {
            println!("   ⚠ Could not configure: {}", stderr.trim());
        }
    }

    println!();
    println!("Setup complete! Use these commands to switch:");
    println!("  halvor k8s direct    - Direct K3s connection (fast, requires network access)");
    println!("  halvor k8s tailscale - Via Tailscale (works from anywhere on tailnet)");

    Ok(())
}

/// Get list of available kubectl contexts
fn get_available_contexts() -> Result<Vec<String>> {
    let output = Command::new("kubectl")
        .args(["config", "get-contexts", "-o", "name"])
        .output()
        .context("Failed to get kubectl contexts")?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.lines().map(|s| s.to_string()).collect())
    } else {
        Ok(Vec::new())
    }
}

/// Switch to a specific context
fn switch_context(name: &str) -> Result<()> {
    let output = Command::new("kubectl")
        .args(["config", "use-context", name])
        .output()
        .context("Failed to switch kubectl context")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to switch context: {}", stderr);
    }

    Ok(())
}
