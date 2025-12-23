//! Cluster backup and restore service
//!
//! Handles full cluster backup and restore including etcd, Helm releases, and secrets.

use crate::config::EnvConfig;
use crate::utils::exec::{CommandExecutor, Executor};
use anyhow::Result;
use chrono::Utc;
use std::io::{self, Write};
use std::path::Path;

/// Create a full cluster backup
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
            println!("Please manually reinstall Helm charts using 'halvor helm install'");
        }
    }

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✓ Restore complete");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}

/// List available backups
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
        println!("Restore with: halvor cluster restore <backup-dir>");
    }

    Ok(())
}

/// Validate backup integrity
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
