//! Configure Tailscale integration for existing K3s cluster

use crate::config::EnvConfig;
use crate::apps::tailscale;
use crate::utils::exec::{CommandExecutor, Executor};
use anyhow::{Context, Result};

/// Configure an existing K3s cluster to use Tailscale
/// This ensures:
/// 1. Tailscale is installed and running
/// 2. K3s service depends on Tailscale
/// 3. K3s uses Tailscale IP for cluster communication
pub fn configure_tailscale_for_k3s(hostname: &str, config: &EnvConfig) -> Result<()> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Configuring Tailscale integration for K3s cluster");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    let exec = Executor::new(hostname, config)
        .with_context(|| format!("Failed to create executor for hostname: {}", hostname))?;
    let is_local = exec.is_local();

    if is_local {
        println!("Target: localhost ({})", hostname);
    } else {
        println!("Target: {} (remote)", hostname);
    }
    println!();

    // Step 1: Ensure Tailscale is installed
    println!("[1/4] Checking Tailscale installation...");
    if !tailscale::is_tailscale_installed(&exec) {
        println!("Tailscale not found. Installing Tailscale...");
        if is_local {
            tailscale::install_tailscale()?;
        } else {
            tailscale::install_tailscale_on_host(hostname, config)?;
        }
        println!("✓ Tailscale installed");
    } else {
        println!("✓ Tailscale is installed");
    }

    // Step 2: Ensure Tailscale is running
    println!();
    println!("[2/4] Checking Tailscale status...");
    let tailscale_status = exec
        .execute_shell("tailscale status --json 2>/dev/null || echo 'not_running'")
        .ok();

    let is_tailscale_running = tailscale_status
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| !s.contains("not_running") && !s.trim().is_empty())
        .unwrap_or(false);

    if !is_tailscale_running {
        anyhow::bail!(
            "Tailscale is not running. Please start Tailscale first:\n\
             sudo tailscale up"
        );
    }

    // Get Tailscale IP
    let tailscale_ip = tailscale::get_tailscale_ip_with_fallback(&exec, hostname, config)?;
    let tailscale_hostname = tailscale::get_tailscale_hostname_remote(&exec)
        .ok()
        .flatten();

    println!("✓ Tailscale is running");
    println!("  IP: {}", tailscale_ip);
    if let Some(ref ts_hostname) = tailscale_hostname {
        println!("  Hostname: {}", ts_hostname);
    }

    // Step 3: Configure K3s service to depend on Tailscale
    println!();
    println!("[3/4] Configuring K3s service dependency on Tailscale...");

    // Check if K3s is installed
    let k3s_service_exists = exec
        .execute_shell(
            "systemctl list-unit-files | grep -q '^k3s.service' && echo exists || echo not_exists",
        )
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim() == "exists")
        .unwrap_or(false);

    if !k3s_service_exists {
        anyhow::bail!("K3s service not found. Please ensure K3s is installed.");
    }

    // Determine service name (k3s or k3s-agent)
    // Check which service is actually running
    let k3s_running = exec
        .execute_shell("sudo systemctl is-active k3s 2>/dev/null && echo active || echo inactive")
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim() == "active")
        .unwrap_or(false);

    let service_name = if k3s_running { "k3s" } else { "k3s-agent" };

    let service_override_dir = format!("/etc/systemd/system/{}.service.d", service_name);
    let override_content = r#"[Unit]
After=tailscale.service
Wants=tailscale.service
Requires=network-online.target
"#;

    // Create override directory
    exec.execute_shell(&format!("sudo mkdir -p {}", service_override_dir))?;

    // Write override file
    let override_file = format!("{}/10-tailscale.conf", service_override_dir);
    exec.write_file(&override_file, override_content.as_bytes())
        .context("Failed to create systemd override")?;

    println!("✓ Created systemd override: {}", override_file);

    // Reload systemd
    exec.execute_shell("sudo systemctl daemon-reload")?;
    println!("✓ Reloaded systemd daemon");

    // Step 4: Update K3s configuration to use Tailscale IP
    println!();
    println!("[4/4] Updating K3s configuration to use Tailscale...");

    let k3s_config_dir = "/etc/rancher/k3s";
    let k3s_config_file = format!("{}/config.yaml", k3s_config_dir);

    // Read existing config if it exists
    let config_yaml = if exec.file_exists(&k3s_config_file).unwrap_or(false) {
        exec.read_file(&k3s_config_file).ok().unwrap_or_default()
    } else {
        String::new()
    };

    // Parse or create YAML config using yaml-rust
    use yaml_rust::{Yaml, YamlEmitter, YamlLoader, yaml::Hash};

    let mut docs = if !config_yaml.is_empty() {
        YamlLoader::load_from_str(&config_yaml).unwrap_or_else(|_| vec![Yaml::Hash(Hash::new())])
    } else {
        vec![Yaml::Hash(Hash::new())]
    };

    // Get existing hash or create new one
    let existing_hash = docs
        .get(0)
        .and_then(|d| d.as_hash())
        .cloned()
        .unwrap_or_else(Hash::new);

    // Create new hash with updated values
    let mut new_hash = existing_hash;

    // Update or add advertise-address
    new_hash.insert(
        Yaml::String("advertise-address".to_string()),
        Yaml::String(tailscale_ip.clone()),
    );

    // Update or add tls-san
    let mut tls_sans = vec![tailscale_ip.clone()];
    if let Some(ref ts_hostname) = tailscale_hostname {
        tls_sans.push(ts_hostname.clone());
    }
    tls_sans.push(hostname.to_string());

    // Remove duplicates
    tls_sans.sort();
    tls_sans.dedup();

    // Convert to YAML array
    let tls_san_array: Vec<Yaml> = tls_sans.into_iter().map(|s| Yaml::String(s)).collect();

    new_hash.insert(
        Yaml::String("tls-san".to_string()),
        Yaml::Array(tls_san_array),
    );

    // Replace the first document with our updated hash
    docs[0] = Yaml::Hash(new_hash);

    // Write updated config
    let mut out_str = String::new();
    let mut emitter = YamlEmitter::new(&mut out_str);
    emitter
        .dump(&docs[0])
        .context("Failed to serialize K3s config")?;

    let updated_yaml = out_str;

    exec.write_file(&k3s_config_file, updated_yaml.as_bytes())
        .context("Failed to write K3s config file")?;

    // Build tls-san list for display
    let mut tls_sans_display = vec![tailscale_ip.clone()];
    if let Some(ref ts_hostname) = tailscale_hostname {
        tls_sans_display.push(ts_hostname.clone());
    }
    tls_sans_display.push(hostname.to_string());
    tls_sans_display.sort();
    tls_sans_display.dedup();

    println!("✓ Updated K3s configuration: {}", k3s_config_file);
    println!("  advertise-address: {}", tailscale_ip);
    println!("  tls-san: {}", tls_sans_display.join(", "));

    // Restart K3s service to apply changes
    println!();
    println!("Restarting K3s service to apply Tailscale configuration...");
    exec.execute_shell(&format!("sudo systemctl restart {}.service", service_name))?;

    // Wait a moment for service to restart
    std::thread::sleep(std::time::Duration::from_secs(5));

    // Verify service is running
    let service_status = exec
        .execute_shell(&format!("sudo systemctl is-active {}", service_name))
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    if service_status == "active" {
        println!("✓ K3s service restarted successfully");
    } else {
        println!("⚠️  Warning: K3s service status: {}", service_status);
        println!(
            "   Please check service logs: sudo journalctl -u {} -n 50",
            service_name
        );
    }

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✓ Tailscale integration configured successfully!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    Ok(())
}
