//! Helm chart management service
//!
//! Handles Helm chart installation, upgrades, and management.

use crate::config::EnvConfig;
use crate::utils::exec::{CommandExecutor, Executor};
use anyhow::{Context, Result};
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

    // Find the chart in the halvor charts directory
    let halvor_dir = crate::config::find_halvor_dir()?;
    let chart_path = halvor_dir.join("charts").join(chart);

    if !chart_path.exists() {
        anyhow::bail!(
            "Chart '{}' not found at {}. Use 'halvor helm charts' to list available charts.",
            chart,
            chart_path.display()
        );
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
        "helm install {} {} --namespace {} --create-namespace",
        release_name,
        chart_path.display(),
        ns
    );

    // Add values file if provided
    if let Some(v) = values {
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

    println!();
    println!(
        "✓ Chart '{}' installed as release '{}'",
        chart, release_name
    );

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
    let halvor_dir = crate::config::find_halvor_dir()?;
    let chart_name = chart_ref.split('-').next().unwrap_or(&chart_ref);
    let chart_path = halvor_dir.join("charts").join(chart_name);

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
