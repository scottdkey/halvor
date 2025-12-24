//! K3s maintenance operations (uninstall, snapshots)

use crate::config::EnvConfig;
use crate::utils::exec::{CommandExecutor, Executor};
use anyhow::{Context, Result};
use std::io::{self, Write};

/// Uninstall K3s from a node
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
    println!("List snapshots with: halvor k3s status");

    Ok(())
}

/// Restore from etcd snapshot
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
