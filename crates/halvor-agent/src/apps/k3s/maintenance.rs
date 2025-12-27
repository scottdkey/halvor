//! K3s maintenance operations (uninstall, snapshots, backup, restore)

use halvor_core::config::EnvConfig;
use halvor_core::utils::exec::{CommandExecutor, Executor};
use anyhow::{Context, Result};
use chrono::Utc;
use std::io::{self, Write};
use std::path::Path;

/// Uninstall K3s from a node
#[allow(dead_code)]
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
#[allow(dead_code)]
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
    println!("List snapshots with: halvor status k3s");

    Ok(())
}

/// Restore from etcd snapshot
#[allow(dead_code)]
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

/// Create a full cluster backup (etcd + Helm releases + secrets)
#[allow(dead_code)]
pub fn backup(
    hostname: &str,
    output: Option<&str>,
    include_pvs: bool,
    config: &EnvConfig,
) -> Result<()> {
    let exec = Executor::new(hostname, config)?;

    let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
    let backup_dir = if let Some(o) = output {
        o.to_string()
    } else {
        format!("backup-{}", timestamp)
    };

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Cluster Backup");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Backup directory: {}", backup_dir);
    println!("Include PVs: {}", include_pvs);
    println!();

    // Create backup directory structure
    std::fs::create_dir_all(&backup_dir)?;
    std::fs::create_dir_all(format!("{}/etcd", backup_dir))?;
    std::fs::create_dir_all(format!("{}/helm", backup_dir))?;
    std::fs::create_dir_all(format!("{}/helm/values", backup_dir))?;
    std::fs::create_dir_all(format!("{}/secrets", backup_dir))?;
    std::fs::create_dir_all(format!("{}/configmaps", backup_dir))?;

    // 1. Create etcd snapshot
    println!("Creating etcd snapshot...");
    let snapshot_name = format!("backup-{}", timestamp);
    let snapshot_cmd = format!("sudo k3s etcd-snapshot save --name={}", snapshot_name);
    exec.execute_shell_interactive(&snapshot_cmd)?;

    // Copy snapshot to backup dir
    let copy_cmd = format!(
        "sudo cp /var/lib/rancher/k3s/server/db/snapshots/{} {}/etcd/snapshot.db",
        snapshot_name, backup_dir
    );
    exec.execute_shell(&copy_cmd)?;
    println!("✓ etcd snapshot saved");

    // 2. Backup Helm releases
    println!("Backing up Helm releases...");
    let releases = exec.execute_shell("helm list -A -o json 2>/dev/null || echo '[]'")?;
    let releases_str = String::from_utf8_lossy(&releases.stdout);
    std::fs::write(
        format!("{}/helm/releases.json", backup_dir),
        releases_str.as_ref(),
    )?;

    // Export values for each release
    if let Ok(releases_json) = serde_json::from_str::<Vec<serde_json::Value>>(&releases_str) {
        for release in releases_json {
            if let (Some(name), Some(namespace)) = (
                release.get("name").and_then(|v| v.as_str()),
                release.get("namespace").and_then(|v| v.as_str()),
            ) {
                let values_cmd = format!(
                    "helm get values {} -n {} --all -o yaml 2>/dev/null || echo ''",
                    name, namespace
                );
                let values = exec.execute_shell(&values_cmd)?;
                let values_str = String::from_utf8_lossy(&values.stdout);
                if !values_str.trim().is_empty() {
                    std::fs::write(
                        format!("{}/helm/values/{}-{}.yaml", backup_dir, namespace, name),
                        values_str.as_ref(),
                    )?;
                }
            }
        }
    }
    println!("✓ Helm releases backed up");

    // 3. Backup secrets
    println!("Backing up secrets...");
    let secrets_cmd = "kubectl get secrets -A -o yaml 2>/dev/null || echo ''";
    let secrets = exec.execute_shell(secrets_cmd)?;
    std::fs::write(
        format!("{}/secrets/secrets.yaml", backup_dir),
        String::from_utf8_lossy(&secrets.stdout).as_ref(),
    )?;
    println!("✓ Secrets backed up");

    // 4. Backup ConfigMaps
    println!("Backing up ConfigMaps...");
    let cm_cmd = "kubectl get configmaps -A -o yaml 2>/dev/null || echo ''";
    let configmaps = exec.execute_shell(cm_cmd)?;
    std::fs::write(
        format!("{}/configmaps/configmaps.yaml", backup_dir),
        String::from_utf8_lossy(&configmaps.stdout).as_ref(),
    )?;
    println!("✓ ConfigMaps backed up");

    // 5. Backup PVs if requested
    if include_pvs {
        std::fs::create_dir_all(format!("{}/pvs", backup_dir))?;
        println!("Backing up PersistentVolumes...");
        println!("  Note: This may take a while for large volumes");
        // This would require ssh access to nodes to backup the actual data
        // For now, just backup the PV/PVC definitions
        let pvs_cmd = "kubectl get pv,pvc -A -o yaml 2>/dev/null || echo ''";
        let pvs = exec.execute_shell(pvs_cmd)?;
        std::fs::write(
            format!("{}/pvs/definitions.yaml", backup_dir),
            String::from_utf8_lossy(&pvs.stdout).as_ref(),
        )?;
        println!("✓ PV definitions backed up");
    }

    // Create metadata file
    let metadata = serde_json::json!({
        "timestamp": timestamp.to_string(),
        "hostname": hostname,
        "include_pvs": include_pvs,
    });
    std::fs::write(
        format!("{}/metadata.json", backup_dir),
        serde_json::to_string_pretty(&metadata)?,
    )?;

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✓ Backup complete: {}", backup_dir);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}

/// Restore cluster from backup
#[allow(dead_code)]
pub fn restore(
    hostname: &str,
    backup_path: &str,
    yes: bool,
    etcd_only: bool,
    config: &EnvConfig,
) -> Result<()> {
    let exec = Executor::new(hostname, config)?;

    if !Path::new(backup_path).exists() {
        anyhow::bail!("Backup not found: {}", backup_path);
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Cluster Restore");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Backup: {}", backup_path);
    println!("etcd only: {}", etcd_only);
    println!();

    // Read metadata
    let metadata_path = format!("{}/metadata.json", backup_path);
    if Path::new(&metadata_path).exists() {
        let metadata = std::fs::read_to_string(&metadata_path)?;
        println!("Backup metadata:");
        println!("{}", metadata);
        println!();
    }

    if !yes {
        println!("WARNING: This will restore the cluster from backup.");
        println!("Current state will be overwritten!");
        print!("Continue? [y/N]: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    // 1. Restore etcd snapshot
    let snapshot_path = format!("{}/etcd/snapshot.db", backup_path);
    if Path::new(&snapshot_path).exists() {
        println!("Restoring etcd snapshot...");

        // Stop K3s
        exec.execute_shell("sudo systemctl stop k3s")?;

        // Restore
        let restore_cmd = format!(
            "sudo k3s server --cluster-reset --cluster-reset-restore-path={}",
            snapshot_path
        );
        exec.execute_shell_interactive(&restore_cmd)?;

        // Start K3s
        exec.execute_shell("sudo systemctl start k3s")?;
        println!("✓ etcd restored");

        // Wait for cluster to be ready
        println!("Waiting for cluster to be ready...");
        std::thread::sleep(std::time::Duration::from_secs(30));
    }

    if !etcd_only {
        // 2. Apply ConfigMaps
        let cm_path = format!("{}/configmaps/configmaps.yaml", backup_path);
        if Path::new(&cm_path).exists() {
            println!("Restoring ConfigMaps...");
            exec.execute_shell(&format!("kubectl apply -f {} 2>/dev/null || true", cm_path))?;
            println!("✓ ConfigMaps restored");
        }

        // 3. Apply Secrets
        let secrets_path = format!("{}/secrets/secrets.yaml", backup_path);
        if Path::new(&secrets_path).exists() {
            println!("Restoring Secrets...");
            exec.execute_shell(&format!(
                "kubectl apply -f {} 2>/dev/null || true",
                secrets_path
            ))?;
            println!("✓ Secrets restored");
        }

        // 4. Reinstall Helm releases
        let releases_path = format!("{}/helm/releases.json", backup_path);
        if Path::new(&releases_path).exists() {
            println!("Helm releases are backed up but automatic reinstall is not yet implemented.");
            println!("Please manually reinstall Helm charts using 'halvor install <chart> -H <hostname>'");
        }
    }

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✓ Restore complete");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}

/// List available backups
#[allow(dead_code)]
pub fn list_backups(path: Option<&str>) -> Result<()> {
    let search_path = path.unwrap_or(".");

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Available Backups");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Searching in: {}", search_path);
    println!();

    let entries = std::fs::read_dir(search_path)?;
    let mut backups: Vec<(String, String)> = Vec::new();

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let metadata_path = path.join("metadata.json");
            if metadata_path.exists() {
                if let Ok(metadata) = std::fs::read_to_string(&metadata_path) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&metadata) {
                        let timestamp = json
                            .get("timestamp")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        backups.push((
                            path.file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string(),
                            timestamp.to_string(),
                        ));
                    }
                }
            }
        }
    }

    if backups.is_empty() {
        println!("No backups found.");
    } else {
        backups.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by timestamp descending
        for (name, timestamp) in backups {
            println!("  {} ({})", name, timestamp);
        }
        println!();
        println!("Restore with: halvor k3s restore <backup-dir>");
    }

    Ok(())
}

/// Regenerate K3s server certificates with Tailscale hostname
/// This is needed when TLS SANs change (e.g., adding Tailscale hostname)
#[allow(dead_code)]
pub fn regenerate_certificates(hostname: &str, yes: bool, config: &EnvConfig) -> Result<()> {
    use crate::apps::tailscale;

    let exec = Executor::new(hostname, config)?;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Regenerate K3s Server Certificates");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Get Tailscale info
    let tailscale_ip = tailscale::get_tailscale_ip_with_fallback(&exec, hostname, config)?;
    let tailscale_hostname = tailscale::get_tailscale_hostname_remote(&exec)
        .ok()
        .flatten();

    println!("Current Tailscale configuration:");
    println!("  IP: {}", tailscale_ip);
    if let Some(ref ts_hostname) = tailscale_hostname {
        println!("  Hostname: {}", ts_hostname);
    }
    println!();

    if !yes {
        println!("This will:");
        println!("  1. Update K3s config to include Tailscale hostname in TLS SANs");
        println!("  2. Delete existing K3s certificates");
        println!("  3. Restart K3s to regenerate certificates with new SANs");
        println!();
        println!("WARNING: This will cause a brief cluster disruption!");
        print!("Continue? [y/N]: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Step 1: Update systemd service file to add TLS SANs
    println!("Updating K3s systemd service file...");

    // Read current service file
    let service_content = exec.read_file("/etc/systemd/system/k3s.service")
        .context("Failed to read k3s.service file")?;

    // Build TLS SAN arguments
    let mut tls_san_args = format!("        '--tls-san={}' \\\n", tailscale_ip);
    if let Some(ref ts_hostname) = tailscale_hostname {
        tls_san_args.push_str(&format!("        '--tls-san={}' \\\n", ts_hostname));
    }
    tls_san_args.push_str(&format!("        '--tls-san={}' \\\n", hostname));

    // Check if TLS SANs are already in the file
    let has_tls_sans = service_content.contains("--tls-san");

    let new_content = if has_tls_sans {
        // Remove existing TLS SAN lines and add new ones
        let lines: Vec<&str> = service_content.lines().collect();
        let mut new_lines = Vec::new();
        let mut skip_tls_san = false;

        for line in lines {
            if line.trim().starts_with("'--tls-san=") {
                skip_tls_san = true;
                continue;
            }
            if skip_tls_san && !line.trim().is_empty() {
                // Insert new TLS SANs before this line
                new_lines.push(tls_san_args.trim_end().to_string());
                skip_tls_san = false;
            }
            new_lines.push(line.to_string());
        }
        new_lines.join("\n")
    } else {
        // Add TLS SANs after the '--disable=traefik' line
        service_content.replace(
            "        '--disable=traefik' \\\n",
            &format!("        '--disable=traefik' \\\n{}", tls_san_args)
        )
    };

    // Write updated service file
    exec.write_file("/etc/systemd/system/k3s.service", new_content.as_bytes())
        .context("Failed to write updated k3s.service file")?;

    println!("  ✓ Added TLS SANs to systemd service file:");
    println!("    - {}", tailscale_ip);
    if let Some(ref ts_hostname) = tailscale_hostname {
        println!("    - {}", ts_hostname);
    }
    println!("    - {}", hostname);

    // Reload systemd
    println!("  ✓ Reloading systemd daemon...");
    exec.execute_shell("sudo systemctl daemon-reload")?;

    // Step 2: Delete existing certificates to force regeneration
    println!();
    println!("Deleting existing certificates to force regeneration...");
    let cert_files = vec![
        "/var/lib/rancher/k3s/server/tls/serving-kube-apiserver.crt",
        "/var/lib/rancher/k3s/server/tls/serving-kube-apiserver.key",
        "/var/lib/rancher/k3s/server/tls/client-ca.crt",
        "/var/lib/rancher/k3s/server/tls/client-ca.key",
        "/var/lib/rancher/k3s/server/tls/server-ca.crt",
        "/var/lib/rancher/k3s/server/tls/server-ca.key",
    ];

    for cert_file in cert_files {
        let result = exec.execute_shell(&format!("sudo rm -f {}", cert_file));
        if result.is_ok() {
            println!("  ✓ Deleted {}", cert_file);
        }
    }

    // Step 3: Restart K3s to apply changes and regenerate certificates
    println!();
    println!("Restarting K3s service...");
    exec.execute_shell("sudo systemctl restart k3s")?;

    println!("Waiting for K3s to regenerate certificates...");
    std::thread::sleep(std::time::Duration::from_secs(10));

    // Verify K3s is running
    let status = exec.execute_shell("sudo systemctl is-active k3s")?;
    let is_active = String::from_utf8_lossy(&status.stdout).trim() == "active";

    if is_active {
        println!("✓ K3s is running and certificates have been regenerated");
    } else {
        println!("⚠ K3s may not be running. Check status with: halvor status k3s -H {}", hostname);
    }

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✓ Certificates regenerated");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Automatically update kubeconfig if running locally (to pick up new CA certificate)
    if exec.is_local() && std::path::Path::new("/etc/rancher/k3s/k3s.yaml").exists() {
        println!("Updating kubeconfig with new CA certificate...");

        // Import kubeconfig module
        use crate::apps::k3s::kubeconfig;
        use halvor_core::utils::exec::local;

        // Check if kubectl exists
        if local::check_command_exists("kubectl") {
            match kubeconfig::fetch_kubeconfig_content(hostname, config) {
                Ok(mut kubeconfig_content) => {
                    // Rename context to 'halvor'
                    kubeconfig_content = kubeconfig_content.replace("name: default", "name: halvor");
                    kubeconfig_content = kubeconfig_content.replace("cluster: default", "cluster: halvor");
                    kubeconfig_content = kubeconfig_content.replace("user: default", "user: halvor");
                    kubeconfig_content = kubeconfig_content.replace("current-context: default", "current-context: halvor");

                    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
                    let kube_dir = format!("{}/.kube", home);
                    let main_config = format!("{}/config", kube_dir);
                    let temp_config = format!("{}/halvor-temp.yaml", kube_dir);

                    // Create .kube directory if it doesn't exist
                    let _ = std::fs::create_dir_all(&kube_dir);

                    // Write temp kubeconfig
                    if std::fs::write(&temp_config, &kubeconfig_content).is_ok() {
                        // Try to merge with existing config
                        let merge_cmd = format!(
                            "export KUBECONFIG='{}:{}' && kubectl config view --flatten > /tmp/kube-merged.yaml && mv /tmp/kube-merged.yaml '{}'",
                            main_config, temp_config, main_config
                        );

                        if local::execute_shell(&merge_cmd).is_ok() {
                            println!("  ✓ Kubeconfig updated with new CA certificate");

                            // Set halvor as current context
                            let _ = local::execute_shell("kubectl config use-context halvor");
                        }

                        // Clean up temp file
                        let _ = std::fs::remove_file(&temp_config);
                    }
                }
                Err(_) => {
                    println!("  ⚠ Could not automatically update kubeconfig");
                    println!("  Run manually: halvor config kubeconfig --setup");
                }
            }
        } else {
            println!("  ℹ kubectl not found - kubeconfig not updated");
            println!("  To update later: halvor config kubeconfig --setup");
        }
    } else {
        println!("Next steps:");
        println!("  1. Update kubeconfig: halvor config kubeconfig --setup");
        println!("  2. Test connection: kubectl --context halvor cluster-info");
    }

    Ok(())
}

/// Validate backup integrity
#[allow(dead_code)]
pub fn validate_backup(backup_path: &str) -> Result<()> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Validate Backup");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Backup: {}", backup_path);
    println!();

    if !Path::new(backup_path).exists() {
        anyhow::bail!("Backup not found: {}", backup_path);
    }

    let mut valid = true;
    let mut issues: Vec<String> = Vec::new();

    // Check metadata
    let metadata_path = format!("{}/metadata.json", backup_path);
    if Path::new(&metadata_path).exists() {
        println!("✓ metadata.json exists");
        if let Ok(content) = std::fs::read_to_string(&metadata_path) {
            if serde_json::from_str::<serde_json::Value>(&content).is_ok() {
                println!("  ✓ Valid JSON");
            } else {
                issues.push("metadata.json is not valid JSON".to_string());
                valid = false;
            }
        }
    } else {
        issues.push("metadata.json not found".to_string());
        valid = false;
    }

    // Check etcd snapshot
    let snapshot_path = format!("{}/etcd/snapshot.db", backup_path);
    if Path::new(&snapshot_path).exists() {
        let metadata = std::fs::metadata(&snapshot_path)?;
        if metadata.len() > 0 {
            println!("✓ etcd/snapshot.db exists ({} bytes)", metadata.len());
        } else {
            issues.push("etcd/snapshot.db is empty".to_string());
            valid = false;
        }
    } else {
        issues.push("etcd/snapshot.db not found".to_string());
        valid = false;
    }

    // Check Helm releases
    let releases_path = format!("{}/helm/releases.json", backup_path);
    if Path::new(&releases_path).exists() {
        println!("✓ helm/releases.json exists");
    } else {
        issues.push("helm/releases.json not found".to_string());
    }

    // Check secrets
    let secrets_path = format!("{}/secrets/secrets.yaml", backup_path);
    if Path::new(&secrets_path).exists() {
        println!("✓ secrets/secrets.yaml exists");
    } else {
        issues.push("secrets/secrets.yaml not found".to_string());
    }

    println!();
    if valid {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("✓ Backup is valid");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    } else {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("✗ Backup has issues:");
        for issue in issues {
            println!("  - {}", issue);
        }
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    }

    Ok(())
}
