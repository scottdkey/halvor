//! Helm chart management service
//!
//! Handles Helm chart installation, upgrades, and management.

use crate::config::EnvConfig;
use crate::utils::exec::{CommandExecutor, Executor};
use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose};
use reqwest;
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::Path;

/// Generate values from environment variables for a chart
fn generate_values_from_env(chart: &str) -> Result<HashMap<String, String>> {
    let mut values = HashMap::new();

    match chart {
        "traefik-public" => {
            let domain = std::env::var("PUBLIC_DOMAIN")
                .context("PUBLIC_DOMAIN environment variable not set (should be in 1Password)")?;
            let acme_email = std::env::var("ACME_EMAIL")
                .context("ACME_EMAIL environment variable not set (should be in 1Password)")?;
            let cf_token = std::env::var("CF_DNS_API_TOKEN").context(
                "CF_DNS_API_TOKEN environment variable not set (should be in 1Password)",
            )?;

            values.insert("domain".to_string(), domain.clone());
            values.insert("acme.email".to_string(), acme_email);
            values.insert("acme.dnsToken".to_string(), cf_token);
            values.insert(
                "dashboard.domain".to_string(),
                format!("traefik.{}", domain),
            );
        }
        "traefik-private" => {
            let domain = std::env::var("PRIVATE_DOMAIN")
                .context("PRIVATE_DOMAIN environment variable not set (should be in 1Password)")?;
            let acme_email = std::env::var("ACME_EMAIL")
                .context("ACME_EMAIL environment variable not set (should be in 1Password)")?;
            let cf_token = std::env::var("CF_DNS_API_TOKEN").context(
                "CF_DNS_API_TOKEN environment variable not set (should be in 1Password)",
            )?;

            values.insert("domain".to_string(), domain.clone());
            values.insert("acme.email".to_string(), acme_email);
            values.insert("acme.dnsToken".to_string(), cf_token);
            values.insert(
                "dashboard.domain".to_string(),
                format!("traefik.{}", domain),
            );
        }
        "gitea" => {
            let domain = std::env::var("GITEA_DOMAIN")
                .or_else(|_| std::env::var("PUBLIC_DOMAIN").map(|d| format!("gitea.{}", d)))
                .or_else(|_| std::env::var("PRIVATE_DOMAIN").map(|d| format!("gitea.{}", d)))
                .context("GITEA_DOMAIN, PUBLIC_DOMAIN, or PRIVATE_DOMAIN environment variable not set (should be in 1Password)")?;

            let root_url =
                std::env::var("GITEA_ROOT_URL").unwrap_or_else(|_| format!("https://{}", domain));

            values.insert("domain".to_string(), domain.clone());
            values.insert("gitea.server.domain".to_string(), domain.clone());
            values.insert("gitea.server.rootUrl".to_string(), root_url);
            values.insert("ingress.hosts[0].host".to_string(), domain);
        }
        "smb-storage" => {
            // SMB storage chart doesn't need server credentials in values
            // The shares are configured statically in values.yaml
            // SMB mounts are set up on nodes via 'halvor smb' command before deploying this chart
            // The chart just creates PVs pointing to the mount points
            // Note: SMB servers (maple and willow) should be configured in .env with:
            //   SMB_maple_HOST, SMB_maple_USERNAME, SMB_maple_PASSWORD, SMB_maple_SHARES
            //   SMB_willow_HOST, SMB_willow_USERNAME, SMB_willow_PASSWORD, SMB_willow_SHARES
            println!(
                "Note: SMB mounts should be set up on cluster nodes (frigg and baulder) using 'halvor smb' before deploying this chart"
            );
        }
        "pia-vpn" => {
            // Note: PIA credentials are created as a Kubernetes Secret separately
            // They are not passed as Helm values for security
            let region = std::env::var("REGION").unwrap_or_default();
            let update_configs = std::env::var("UPDATE_CONFIGS")
                .unwrap_or_else(|_| "true".to_string())
                .parse::<bool>()
                .unwrap_or(true);
            let proxy_port = std::env::var("PROXY_PORT").unwrap_or_else(|_| "8888".to_string());
            let debug = std::env::var("DEBUG")
                .unwrap_or_else(|_| "false".to_string())
                .parse::<bool>()
                .unwrap_or(false);

            // Set image tag based on development mode
            let is_dev = std::env::var("HALVOR_ENV")
                .map(|v| v.to_lowercase() == "development")
                .unwrap_or(false);
            let image_tag = if is_dev { "experimental" } else { "latest" };
            values.insert("image.tag".to_string(), image_tag.to_string());

            values.insert("vpn.region".to_string(), region);
            values.insert("vpn.updateConfigs".to_string(), update_configs.to_string());
            values.insert("vpn.proxyPort".to_string(), proxy_port);
            values.insert("vpn.debug".to_string(), debug.to_string());
        }
        "halvor-server" => {
            // Set image tag based on development mode
            let is_dev = std::env::var("HALVOR_ENV")
                .map(|v| v.to_lowercase() == "development")
                .unwrap_or(false);
            let image_tag = if is_dev { "experimental" } else { "latest" };
            values.insert("image.tag".to_string(), image_tag.to_string());
        }
        _ => {
            // For other charts, try to load common env vars
            if let Ok(domain) = std::env::var("PUBLIC_DOMAIN") {
                values.insert("domain".to_string(), domain);
            }
        }
    }

    Ok(values)
}

/// Convert values map to --set flags for Helm
fn values_to_set_flags(values: &HashMap<String, String>) -> Vec<String> {
    let mut flags = Vec::new();
    for (key, value) in values {
        // Escape special characters in values for --set
        let escaped_value = value.replace("'", "''").replace(",", "\\,");
        flags.push(format!("{}='{}'", key, escaped_value));
    }
    flags
}

/// Check if Kubernetes cluster is accessible
fn check_cluster_available<E: CommandExecutor>(exec: &E) -> Result<()> {
    // Try kubectl first (if kubeconfig is set up)
    let kubectl_check = exec.execute_shell("kubectl cluster-info --request-timeout=5s 2>&1");
    
    if let Ok(output) = kubectl_check {
        if output.status.success() {
            // Cluster is accessible via kubectl
            return Ok(());
        }
    }
    
    // Try k3s kubectl (k3s provides kubectl via k3s kubectl)
    let k3s_kubectl_check = exec.execute_shell("sudo k3s kubectl cluster-info --request-timeout=5s 2>&1");
    
    if let Ok(output) = k3s_kubectl_check {
        if output.status.success() {
            // Cluster is accessible via k3s kubectl
            return Ok(());
        }
    }
    
    // Check if k3s is installed but cluster might not be initialized
    let k3s_check = exec.execute_shell("k3s --version 2>&1 || echo 'not_installed'");
    if let Ok(k3s_output) = k3s_check {
        let k3s_str = String::from_utf8_lossy(&k3s_output.stdout);
        if k3s_str.contains("not_installed") {
            anyhow::bail!(
                "K3s is not installed. Please initialize a cluster first:\n  halvor init -H <hostname>"
            );
        }
    }
    
    anyhow::bail!(
        "Kubernetes cluster is not accessible. Please ensure:\n  - K3s cluster is initialized (halvor init -H <hostname>)\n  - Cluster is running and healthy\n  - You're connecting to the correct host"
    )
}

/// Install a Helm chart
pub fn install_chart(
    hostname: &str,
    chart: &str,
    name: Option<&str>,
    namespace: Option<&str>,
    values: Option<&str>,
    set: &[String],
    config: &EnvConfig,
) -> Result<()> {
    let exec = Executor::new(hostname, config)?;
    let release_name = name.unwrap_or(chart);
    let ns = namespace.unwrap_or("default");

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Install Helm Chart");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Chart: {}", chart);
    println!("Release: {}", release_name);
    println!("Namespace: {}", ns);
    println!();

    // Validate cluster is available before proceeding
    println!("Checking cluster availability...");
    check_cluster_available(&exec)?;
    println!("✓ Cluster is accessible");
    println!();

    // Try to find chart locally first, then fall back to GitHub
    let (chart_path, is_temp_file) = {
        let halvor_dir = crate::config::find_halvor_dir()?;
        let local_path = halvor_dir.join("charts").join(chart);

        if local_path.exists() {
            (local_path.to_string_lossy().to_string(), false)
        } else {
            // Try to install from GitHub releases
            println!("  Chart not found locally, attempting to install from GitHub...");
            let chart_url = format!(
                "https://github.com/scottdkey/halvor/releases/download/charts-latest/{}-0.1.0.tgz",
                chart
            );

            // Download and use the chart from GitHub
            let temp_dir = std::env::temp_dir();
            let chart_tgz = temp_dir.join(format!("{}-0.1.0.tgz", chart));

            // Download the chart (follow redirects - GitHub releases use redirects)
            let client = reqwest::blocking::Client::builder()
                .redirect(reqwest::redirect::Policy::limited(10))
                .build()
                .context("Failed to create HTTP client")?;
            let response = client.get(&chart_url).send();

            match response {
                Ok(resp) if resp.status().is_success() => {
                    let bytes = resp
                        .bytes()
                        .context("Failed to read chart content from GitHub")?;
                    let mut file = std::fs::File::create(&chart_tgz)
                        .context("Failed to create temporary chart file")?;
                    std::io::copy(&mut bytes.as_ref(), &mut file)
                        .context("Failed to write chart file")?;
                    println!("  ✓ Downloaded chart from GitHub");
                    (chart_tgz.to_string_lossy().to_string(), true)
                }
                Ok(resp) => {
                    let status = resp.status();
                    anyhow::bail!(
                        "Chart '{}' not found locally at {} and could not be downloaded from {} (HTTP {}). Use 'halvor helm charts' to list available charts.",
                        chart,
                        local_path.display(),
                        chart_url,
                        status
                    );
                }
                Err(e) => {
                    anyhow::bail!(
                        "Chart '{}' not found locally at {} and could not be downloaded from {} (Error: {}). Use 'halvor helm charts' to list available charts.",
                        chart,
                        local_path.display(),
                        chart_url,
                        e
                    );
                }
            }
        }
    };

    // Create Kubernetes Secret for charts that need it (e.g., pia-vpn)
    if chart == "pia-vpn" {
        create_pia_vpn_secret(hostname, &release_name, &ns, config)?;
    }

    // Generate values from environment variables if no values file provided
    let mut env_set_flags = Vec::new();
    if values.is_none() && set.is_empty() {
        println!("Generating values from environment variables...");
        let env_values = generate_values_from_env(chart)?;
        env_set_flags = values_to_set_flags(&env_values);
        println!("✓ Generated values from environment variables");
    }

    // Build helm install command
    let mut cmd = format!(
        "helm install {} {} --namespace {} --create-namespace --wait --timeout 10m",
        release_name, chart_path, ns
    );

    // Add values file if provided
    if let Some(v) = values {
        let halvor_dir = crate::config::find_halvor_dir()?;
        let values_path = if Path::new(v).is_absolute() {
            v.to_string()
        } else {
            halvor_dir.join(v).to_string_lossy().to_string()
        };
        cmd.push_str(&format!(" -f {}", values_path));
    }

    // Add --set values from environment variables
    for s in &env_set_flags {
        cmd.push_str(&format!(" --set {}", s));
    }

    // Add --set values from command line
    for s in set {
        cmd.push_str(&format!(" --set {}", s));
    }

    println!("Running: {}", cmd);
    println!();

    exec.execute_shell_interactive(&cmd)
        .context("Helm install failed")?;

    // Clean up temporary chart file if we downloaded it
    if is_temp_file {
        if let Err(e) = std::fs::remove_file(&chart_path) {
            eprintln!(
                "  ⚠ Warning: Failed to clean up temporary chart file: {}",
                e
            );
        }
    }

    println!();
    println!(
        "✓ Chart '{}' installed as release '{}'",
        chart, release_name
    );

    // Wait for deployment/pods to be ready
    println!();
    println!("Waiting for pods to be ready...");

    // Try to wait for deployment first (most common case)
    let deployment_wait_cmd = format!(
        "kubectl wait --for=condition=available --timeout=10m deployment/{} -n {}",
        release_name, ns
    );

    let deployment_wait_result = exec.execute_shell(&deployment_wait_cmd);

    if let Ok(output) = deployment_wait_result {
        if output.status.success() {
            println!("✓ Deployment is available and all pods are ready");
        } else {
            // Fallback: wait for pods by label selector
            println!("  Waiting for pods by label selector...");
            let pod_wait_cmd = format!(
                "kubectl wait --for=condition=ready --timeout=10m pod -l app.kubernetes.io/instance={} -n {}",
                release_name, ns
            );

            if let Ok(pod_output) = exec.execute_shell(&pod_wait_cmd) {
                if pod_output.status.success() {
                    println!("✓ All pods are ready");
                } else {
                    // Show pod status for debugging
                    let status_cmd = format!(
                        "kubectl get pods -n {} -l app.kubernetes.io/instance={}",
                        ns, release_name
                    );
                    if let Ok(status_output) = exec.execute_shell(&status_cmd) {
                        let status = String::from_utf8_lossy(&status_output.stdout);
                        if !status.trim().is_empty() {
                            println!("  Current pod status:");
                            println!("{}", status);
                        }
                    }
                    println!(
                        "  ⚠ Some pods may still be initializing. Check status with: kubectl get pods -n {}",
                        ns
                    );
                }
            }
        }
    } else {
        // If kubectl wait fails, at least show pod status
        let status_cmd = format!(
            "kubectl get pods -n {} -l app.kubernetes.io/instance={}",
            ns, release_name
        );
        if let Ok(status_output) = exec.execute_shell(&status_cmd) {
            let status = String::from_utf8_lossy(&status_output.stdout);
            if !status.trim().is_empty() {
                println!("  Current pod status:");
                println!("{}", status);
            }
        }
    }

    Ok(())
}

/// Upgrade a Helm release
pub fn upgrade_release(
    hostname: &str,
    release: &str,
    values: Option<&str>,
    set: &[String],
    config: &EnvConfig,
) -> Result<()> {
    let exec = Executor::new(hostname, config)?;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Upgrade Helm Release");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Release: {}", release);
    println!();

    // Get the chart name from the release
    let info = exec.execute_shell(&format!(
        "helm get metadata {} -o json 2>/dev/null | jq -r '.chart'",
        release
    ))?;
    let chart_ref = String::from_utf8_lossy(&info.stdout).trim().to_string();

    if chart_ref.is_empty() || chart_ref == "null" {
        anyhow::bail!("Release '{}' not found", release);
    }

    // Find the chart in halvor charts directory
    // chart_ref format is typically "chart-name" or "chart-name-version" (e.g., "pia-vpn" or "pia-vpn-0.1.0")
    // We need to extract just the chart name by removing the version part if present
    let halvor_dir = crate::config::find_halvor_dir()?;
    let chart_name = {
        // Check if the last segment looks like a version (e.g., "0.1.0")
        let parts: Vec<&str> = chart_ref.split('-').collect();
        if parts.len() > 1 {
            let last_part = parts.last().unwrap();
            // Check if last part looks like a version (contains digits and dots)
            if last_part.chars().any(|c| c.is_ascii_digit())
                && last_part.chars().all(|c| c.is_ascii_digit() || c == '.')
            {
                // Last part is a version, join all but last
                parts[..parts.len() - 1].join("-")
            } else {
                // Not a version, use the full chart_ref
                chart_ref.clone()
            }
        } else {
            // Single part, use as-is
            chart_ref.clone()
        }
    };
    let chart_path = halvor_dir.join("charts").join(&chart_name);

    let mut cmd = format!("helm upgrade {} {}", release, chart_path.display());

    if let Some(v) = values {
        let values_path = if Path::new(v).is_absolute() {
            v.to_string()
        } else {
            halvor_dir.join(v).to_string_lossy().to_string()
        };
        cmd.push_str(&format!(" -f {}", values_path));
    }

    for s in set {
        cmd.push_str(&format!(" --set {}", s));
    }

    println!("Running: {}", cmd);
    println!();

    exec.execute_shell_interactive(&cmd)
        .context("Helm upgrade failed")?;

    println!();
    println!("✓ Release '{}' upgraded", release);

    Ok(())
}

/// Uninstall a Helm release
pub fn uninstall_release(
    hostname: &str,
    release: &str,
    yes: bool,
    config: &EnvConfig,
) -> Result<()> {
    let exec = Executor::new(hostname, config)?;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Uninstall Helm Release");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Release: {}", release);
    println!();

    if !yes {
        print!(
            "This will uninstall the release '{}'. Continue? [y/N]: ",
            release
        );
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    let cmd = format!("helm uninstall {}", release);
    exec.execute_shell_interactive(&cmd)
        .context("Helm uninstall failed")?;

    println!();
    println!("✓ Release '{}' uninstalled", release);

    Ok(())
}

/// List Helm releases
pub fn list_releases(
    hostname: &str,
    all_namespaces: bool,
    namespace: Option<&str>,
    config: &EnvConfig,
) -> Result<()> {
    let exec = Executor::new(hostname, config)?;

    let mut cmd = "helm list".to_string();

    if all_namespaces {
        cmd.push_str(" --all-namespaces");
    } else if let Some(ns) = namespace {
        cmd.push_str(&format!(" --namespace {}", ns));
    }

    let result = exec.execute_shell(&cmd)?;
    println!("{}", String::from_utf8_lossy(&result.stdout));

    Ok(())
}

/// Create Kubernetes Secret for PIA VPN credentials from environment variables
fn create_pia_vpn_secret(
    hostname: &str,
    release_name: &str,
    namespace: &str,
    config: &EnvConfig,
) -> Result<()> {
    let exec = Executor::new(hostname, config)?;
    let secret_name = format!("{}-credentials", release_name);

    println!("Creating Kubernetes Secret for PIA VPN credentials...");

    // Load credentials from environment variables
    let pia_username = std::env::var("PIA_USERNAME")
        .context("PIA_USERNAME environment variable not set (should be in 1Password)")?;
    let pia_password = std::env::var("PIA_PASSWORD")
        .context("PIA_PASSWORD environment variable not set (should be in 1Password)")?;

    // Base64 encode the credentials
    let username_b64 = general_purpose::STANDARD.encode(pia_username);
    let password_b64 = general_purpose::STANDARD.encode(pia_password);

    // Create secret YAML
    let secret_yaml = format!(
        r#"apiVersion: v1
kind: Secret
metadata:
  name: {}
  namespace: {}
type: Opaque
data:
  pia-username: {}
  pia-password: {}
"#,
        secret_name, namespace, username_b64, password_b64
    );

    // Check if secret already exists
    let check_cmd = format!(
        "kubectl get secret {} -n {} --ignore-not-found -o name 2>/dev/null",
        secret_name, namespace
    );
    let check_result = exec.execute_shell(&check_cmd)?;
    let secret_exists = !String::from_utf8_lossy(&check_result.stdout)
        .trim()
        .is_empty();

    if secret_exists {
        println!("  Secret '{}' already exists, updating...", secret_name);
        // Delete existing secret first
        let delete_cmd = format!(
            "kubectl delete secret {} -n {} --ignore-not-found",
            secret_name, namespace
        );
        exec.execute_shell(&delete_cmd)?;
    }

    // Create secret using kubectl apply
    let temp_file = format!("/tmp/pia-vpn-secret-{}.yaml", release_name);
    exec.write_file(&temp_file, secret_yaml.as_bytes())
        .context("Failed to write secret YAML to temporary file")?;

    let apply_cmd = format!("kubectl apply -f {}", temp_file);
    exec.execute_shell_interactive(&apply_cmd)
        .context("Failed to create Kubernetes Secret")?;

    // Clean up temp file
    let _ = exec.execute_shell(&format!("rm -f {}", temp_file));

    println!(
        "  ✓ Secret '{}' created in namespace '{}'",
        secret_name, namespace
    );
    println!();

    Ok(())
}

/// List available charts in the halvor repo
pub fn list_charts() -> Result<()> {
    let halvor_dir = crate::config::find_halvor_dir()?;
    let charts_dir = halvor_dir.join("charts");

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Available Helm Charts");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    if !charts_dir.exists() {
        println!("No charts directory found at {}", charts_dir.display());
        return Ok(());
    }

    let entries = std::fs::read_dir(&charts_dir)?;
    let mut charts: Vec<String> = Vec::new();

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(name) = path.file_name() {
                let name_str = name.to_string_lossy().to_string();
                // Check if it has a Chart.yaml
                if path.join("Chart.yaml").exists() {
                    charts.push(name_str);
                }
            }
        }
    }

    charts.sort();

    if charts.is_empty() {
        println!("No charts found in {}", charts_dir.display());
    } else {
        for chart in &charts {
            println!("  - {}", chart);
        }
        println!();
        println!("Install with: halvor helm install <chart>");
    }

    Ok(())
}

/// Export values from a running release
pub fn export_values(
    hostname: &str,
    release: &str,
    output: Option<&str>,
    config: &EnvConfig,
) -> Result<()> {
    let exec = Executor::new(hostname, config)?;

    let cmd = format!("helm get values {} --all", release);
    let result = exec.execute_shell(&cmd)?;

    if !result.status.success() {
        anyhow::bail!("Failed to get values for release '{}'", release);
    }

    let values = String::from_utf8_lossy(&result.stdout);

    if let Some(path) = output {
        std::fs::write(path, values.as_ref())
            .with_context(|| format!("Failed to write values to {}", path))?;
        println!("✓ Values exported to {}", path);
    } else {
        println!("{}", values);
    }

    Ok(())
}
