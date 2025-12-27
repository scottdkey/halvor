use crate::commands::config::{ConfigCommands, CreateConfigCommands, DbCommands, MigrateCommands};
use crate::config::{EnvConfig, find_halvor_dir, load_env_config};
use crate::db;
use crate::utils::exec::{CommandExecutor, Executor, local};
use anyhow::{Context, Result};

// Utility functions for hostname operations

/// Get the current machine's hostname
pub fn get_current_hostname() -> Result<String> {
    use std::process::Command;

    // Try hostname command first (works on most Unix systems)
    if let Ok(output) = Command::new("hostname").output() {
        if output.status.success() {
            if let Ok(hostname) = String::from_utf8(output.stdout) {
                return Ok(hostname.trim().to_string());
            }
        }
    }

    // Fallback to whoami crate or environment variable
    #[cfg(unix)]
    {
        if let Ok(hostname) = std::env::var("HOSTNAME") {
            return Ok(hostname);
        }
        if let Ok(hostname) = std::fs::read_to_string("/etc/hostname") {
            return Ok(hostname.trim().to_string());
        }
    }

    anyhow::bail!("Could not determine current hostname")
}

/// Normalize hostname for config lookup (lowercase, handle TLDs)
pub fn normalize_hostname(hostname: &str) -> String {
    let mut normalized = hostname.to_lowercase();

    // Remove common TLDs for matching (e.g., "frigg.local" -> "frigg")
    // But keep Tailscale domains (e.g., "frigg.bombay-pinecone.ts.net" -> "frigg")
    if normalized.ends_with(".local") {
        normalized = normalized.trim_end_matches(".local").to_string();
    }

    normalized
}

/// Find hostname in config with normalization
pub fn find_hostname_in_config(hostname: &str, config: &EnvConfig) -> Option<String> {
    let normalized = normalize_hostname(hostname);

    // Try exact match first
    if config.hosts.contains_key(hostname) {
        return Some(hostname.to_string());
    }

    // Try normalized match
    if config.hosts.contains_key(&normalized) {
        return Some(normalized);
    }

    // Try case-insensitive match
    for (key, _) in &config.hosts {
        if key.eq_ignore_ascii_case(hostname) || normalize_hostname(key) == normalized {
            return Some(key.clone());
        }
    }

    // Try matching base hostname (before first dot)
    let base = hostname.split('.').next().unwrap_or(hostname);
    let normalized_base = normalize_hostname(base);

    for (key, _) in &config.hosts {
        let key_base = key.split('.').next().unwrap_or(key);
        if normalize_hostname(key_base) == normalized_base {
            return Some(key.clone());
        }
    }

    None
}

// Command handlers

pub fn handle_config_command(
    _arg: Option<&str>,
    _verbose: bool,
    _db: bool,
    command: Option<&ConfigCommands>,
) -> Result<()> {
    if let Some(cmd) = command {
        match cmd {
            ConfigCommands::List => {
                show_config_list()?;
            }
            ConfigCommands::Init => {
                anyhow::bail!("Use 'halvor init' command instead of 'halvor config init'");
            }
            ConfigCommands::SetEnv { path } => {
                crate::config::config_manager::set_env_file_path(&std::path::PathBuf::from(path))?;
                println!("✓ Environment file path set to: {}", path);
            }
            ConfigCommands::SetStable => {
                crate::config::config_manager::set_release_channel(
                    crate::config::config_manager::ReleaseChannel::Stable,
                )?;
                println!("✓ Release channel set to stable");
            }
            ConfigCommands::SetExperimental => {
                crate::config::config_manager::set_release_channel(
                    crate::config::config_manager::ReleaseChannel::Experimental,
                )?;
                println!("✓ Release channel set to experimental");
            }
            ConfigCommands::Create { command: _command } => {
                handle_create_command(_command)?;
            }
            ConfigCommands::Env => {
                anyhow::bail!("Env command not yet fully implemented");
            }
            ConfigCommands::SetBackup {
                hostname: _hostname,
            } => {
                anyhow::bail!("SetBackup command not yet fully implemented");
            }
            ConfigCommands::Commit => {
                anyhow::bail!("Commit command not yet fully implemented");
            }
            ConfigCommands::Backup => {
                anyhow::bail!("Backup command not yet fully implemented");
            }
            ConfigCommands::Delete { from_env } => {
                anyhow::bail!(
                    "Delete command not yet fully implemented (from_env: {})",
                    from_env
                );
            }
            ConfigCommands::Ip { value } => {
                anyhow::bail!("Ip command not yet fully implemented (value: {})", value);
            }
            ConfigCommands::Hostname { value } => {
                anyhow::bail!(
                    "Hostname command not yet fully implemented (value: {})",
                    value
                );
            }
            ConfigCommands::BackupPath { value } => {
                anyhow::bail!(
                    "BackupPath command not yet fully implemented (value: {})",
                    value
                );
            }
            ConfigCommands::Diff => {
                show_config_diff()?;
            }
            ConfigCommands::Kubeconfig {
                setup,
                diagnose,
                hostname,
            } => {
                handle_kubeconfig_command(*setup, *diagnose, hostname.as_deref())?;
            }
            ConfigCommands::Regenerate { hostname, yes } => {
                handle_regenerate_command(hostname.as_deref(), *yes)?;
            }
        }
    } else {
        // No subcommand - show current config
        show_config_list()?;
    }
    Ok(())
}

pub fn handle_db_command(command: DbCommands) -> Result<()> {
    match command {
        DbCommands::Backup { path } => {
            anyhow::bail!("Database backup not yet implemented (path: {:?})", path);
        }
        DbCommands::Generate => {
            db::core::generator::generate_structs()?;
        }
        DbCommands::Migrate { command } => {
            if let Some(cmd) = command {
                match cmd {
                    MigrateCommands::Up => {
                        db::migrate::migrate_up()?;
                    }
                    MigrateCommands::Down => {
                        db::migrate::migrate_down()?;
                    }
                    MigrateCommands::List => {
                        db::migrate::migrate_list()?;
                    }
                    MigrateCommands::Generate { description } => {
                        db::migrate::generate_migration(description)?;
                    }
                    MigrateCommands::GenerateShort { description } => {
                        db::migrate::generate_migration(description)?;
                    }
                }
            } else {
                // No subcommand - run all pending migrations
                db::migrate::migrate_all()?;
            }
        }
        DbCommands::Sync => {
            anyhow::bail!("Sync command not yet fully implemented");
        }
        DbCommands::Restore => {
            anyhow::bail!("Restore command not yet implemented");
        }
    }
    Ok(())
}

fn handle_create_command(command: &CreateConfigCommands) -> Result<()> {
    match command {
        CreateConfigCommands::App => {
            anyhow::bail!("Create App command not yet implemented");
        }
        CreateConfigCommands::Smb { server_name } => {
            anyhow::bail!(
                "Create Smb command not yet implemented (server_name: {:?})",
                server_name
            );
        }
        CreateConfigCommands::Ssh { hostname } => {
            anyhow::bail!(
                "Create Ssh command not yet implemented (hostname: {:?})",
                hostname
            );
        }
    }
}

fn show_config_list() -> Result<()> {
    let halvor_dir = find_halvor_dir()?;
    let config = load_env_config(&halvor_dir)?;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Configuration");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    println!("Hosts:");
    if config.hosts.is_empty() {
        println!("  (no hosts configured)");
    } else {
        for (name, host_config) in &config.hosts {
            println!("  {}", name);
            if let Some(ip) = &host_config.ip {
                println!("    IP: {}", ip);
            }
            if let Some(hostname) = &host_config.hostname {
                println!("    Hostname: {}", hostname);
            }
            if let Some(backup_path) = &host_config.backup_path {
                println!("    Backup Path: {}", backup_path);
            }
        }
    }

    println!();
    println!("SMB Servers:");
    if config.smb_servers.is_empty() {
        println!("  (no SMB servers configured)");
    } else {
        for (name, smb_config) in &config.smb_servers {
            println!("  {}", name);
            println!("    Host: {}", smb_config.host);
            println!("    Shares: {}", smb_config.shares.join(", "));
        }
    }

    println!();
    Ok(())
}

fn show_config_diff() -> Result<()> {
    anyhow::bail!("Config diff not yet implemented");
}

/// Handle kubeconfig command - print or setup kubectl context
fn handle_kubeconfig_command(setup: bool, diagnose: bool, hostname: Option<&str>) -> Result<()> {
    use crate::services::k3s::kubeconfig;
    use std::io::{self, Write};

    let halvor_dir = find_halvor_dir()?;
    let config = load_env_config(&halvor_dir)?;

    // Determine primary hostname - use provided, or prompt user
    let primary_hostname = if let Some(h) = hostname {
        h.to_string()
    } else {
        // Always prompt user to select a host
        if config.hosts.is_empty() {
            anyhow::bail!(
                "No hosts found in configuration. Please add a host to your config or specify hostname with -H"
            );
        }

        println!("Available hosts:");
        let hosts: Vec<String> = config.hosts.keys().cloned().collect();
        for (idx, host) in hosts.iter().enumerate() {
            println!("  {}. {}", idx + 1, host);
        }
        println!();
        print!("Select host to use for kubeconfig (enter number or hostname): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        // Try to parse as number first
        if let Ok(num) = input.parse::<usize>() {
            if num > 0 && num <= hosts.len() {
                hosts[num - 1].clone()
            } else {
                anyhow::bail!(
                    "Invalid selection: {} (must be between 1 and {})",
                    num,
                    hosts.len()
                );
            }
        } else {
            // Try to find hostname in config
            find_hostname_in_config(input, &config).ok_or_else(|| {
                anyhow::anyhow!(
                    "Host '{}' not found in configuration. Available hosts: {}",
                    input,
                    hosts.join(", ")
                )
            })?
        }
    };

    // Run diagnostics if requested
    if diagnose {
        return run_kubeconfig_diagnostics(&primary_hostname, &config);
    }

    if setup {
        // Setup local kubectl context
        println!("Setting up kubectl context 'halvor'...");
        println!();

        // Check if kubectl exists
        if !local::check_command_exists("kubectl") {
            println!("  ✗ kubectl not found. Install kubectl first:");
            println!("     macOS: brew install kubectl");
            println!("     Linux: See https://kubernetes.io/docs/tasks/tools/");
            return Ok(());
        }

        // Check if K3s on primary node needs Tailscale configuration
        println!("  Checking K3s configuration on {}...", primary_hostname);
        let exec = Executor::new(&primary_hostname, &config).ok();
        if let Some(ref exec_ref) = exec {
            if !exec_ref.is_local() {
                // Check if K3s config has Tailscale hostname in tls-san
                let config_check = exec_ref
                    .execute_shell("sudo cat /etc/rancher/k3s/config.yaml 2>/dev/null || echo ''")
                    .ok();

                let needs_config = if let Some(config_output) = config_check {
                    let config_str = String::from_utf8_lossy(&config_output.stdout);
                    // Check if config has tls-san with .ts.net hostname
                    let has_ts_hostname = config_str.contains(".ts.net");
                    let has_tls_san = config_str.contains("tls-san");
                    !has_tls_san || !has_ts_hostname
                } else {
                    true // If we can't read config, assume it needs configuration
                };

                if needs_config {
                    println!("  ⚠️  K3s configuration may be missing Tailscale hostname");
                    println!("  Configuring K3s with Tailscale settings...");
                    println!();

                    // Configure Tailscale for K3s
                    if let Err(e) = crate::services::k3s::configure_tailscale_for_k3s(
                        &primary_hostname,
                        &config,
                    ) {
                        println!(
                            "  ⚠️  Warning: Failed to configure Tailscale for K3s: {}",
                            e
                        );
                        println!("  Continuing anyway...");
                        println!();
                    } else {
                        println!("  ✓ K3s configured with Tailscale");
                        println!("  Waiting for K3s to restart...");
                        std::thread::sleep(std::time::Duration::from_secs(3));
                        println!();
                    }
                } else {
                    println!("  ✓ K3s configuration looks good");
                    println!();
                }
            }
        }

        // Fetch kubeconfig
        let mut kubeconfig_content =
            kubeconfig::fetch_kubeconfig_content(&primary_hostname, &config)?;

        // Simple string replacement to rename context and cluster to 'halvor'
        kubeconfig_content = kubeconfig_content.replace("name: default", "name: halvor");
        kubeconfig_content = kubeconfig_content.replace("cluster: default", "cluster: halvor");
        kubeconfig_content = kubeconfig_content.replace("user: default", "user: halvor");
        kubeconfig_content =
            kubeconfig_content.replace("current-context: default", "current-context: halvor");

        // Create temp file for kubeconfig
        let home = std::env::var("HOME").context("HOME environment variable not set")?;
        let kube_dir = format!("{}/.kube", home);
        std::fs::create_dir_all(&kube_dir).context("Failed to create ~/.kube directory")?;

        let main_config = format!("{}/.kube/config", home);
        let temp_config = format!("{}/.kube/halvor-temp.yaml", home);

        // Write processed kubeconfig to temp file
        std::fs::write(&temp_config, &kubeconfig_content)
            .context("Failed to write temporary kubeconfig")?;

        println!("  Configuring context 'halvor'...");

        // Backup existing config if it exists
        if std::path::Path::new(&main_config).exists() {
            let backup = format!("{}.backup-{}", main_config, chrono::Utc::now().timestamp());
            std::fs::copy(&main_config, &backup).context("Failed to backup existing kubeconfig")?;
            println!("  Created backup at {}", backup);
        }

        // Merge configs - use kubectl config view with KUBECONFIG env var
        let merge_cmd = format!(
            "export KUBECONFIG='{}:{}' && kubectl config view --flatten > /tmp/kube-merged.yaml && mv /tmp/kube-merged.yaml '{}'",
            main_config, temp_config, main_config
        );

        let merge_result = local::execute_shell(&merge_cmd);
        let mut merge_success = merge_result
            .as_ref()
            .map(|r| r.status.success())
            .unwrap_or(false);

        if merge_success {
            println!("  ✓ Merged kubeconfig with existing config");
        } else {
            // If merge fails, handle based on whether config exists
            if !std::path::Path::new(&main_config).exists() {
                // No existing config - just copy temp config
                println!("  No existing kubeconfig found, creating new one...");
                std::fs::copy(&temp_config, &main_config)
                    .context("Failed to copy kubeconfig to ~/.kube/config")?;
                merge_success = true; // Copying is considered success
            } else {
                // Existing config - try alternative merge method
                println!("  Merge failed, trying alternative method...");

                // Check what the error was
                if let Err(e) = &merge_result {
                    println!("  Error: {}", e);
                } else if let Ok(output) = &merge_result {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if !stderr.trim().is_empty() {
                        println!("  Error output: {}", stderr.trim());
                    }
                }

                // Use KUBECONFIG env var without export (works in subshell)
                let alt_merge_cmd = format!(
                    "KUBECONFIG='{}:{}' kubectl config view --flatten > /tmp/kube-merged-alt.yaml 2>&1 && mv /tmp/kube-merged-alt.yaml '{}'",
                    main_config, temp_config, main_config
                );
                let alt_result = local::execute_shell(&alt_merge_cmd);

                if let Err(e) = &alt_result {
                    println!("  Alternative method also failed: {}", e);
                } else if let Ok(output) = &alt_result {
                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        println!("  Alternative method failed");
                        if !stderr.trim().is_empty() {
                            println!("  Error: {}", stderr.trim());
                        }
                        if !stdout.trim().is_empty() && stdout != stderr {
                            println!("  Output: {}", stdout.trim());
                        }
                    }
                }

                let alt_success = alt_result
                    .as_ref()
                    .map(|r| r.status.success())
                    .unwrap_or(false);

                if !alt_success {
                    // Last resort: write the halvor config to a permanent location
                    println!(
                        "  All merge methods failed, writing halvor config to permanent location..."
                    );

                    // Write to a permanent halvor-specific config file
                    let halvor_config = format!("{}/.kube/halvor.yaml", home);
                    std::fs::copy(&temp_config, &halvor_config)
                        .context("Failed to write halvor kubeconfig to permanent location")?;
                    println!("  ✓ Written halvor kubeconfig to: {}", halvor_config);

                    // Try one more time with the permanent file
                    let final_merge_cmd = format!(
                        "KUBECONFIG='{}:{}' kubectl config view --flatten > /tmp/kube-final-merge.yaml 2>&1 && mv /tmp/kube-final-merge.yaml '{}'",
                        main_config, halvor_config, main_config
                    );
                    let final_result = local::execute_shell(&final_merge_cmd);

                    if let Ok(output) = final_result {
                        if output.status.success() {
                            merge_success = true;
                            println!("  ✓ Successfully merged using permanent halvor config file");
                        } else {
                            println!("  ⚠️  Final merge attempt also failed");
                            println!(
                                "  The halvor kubeconfig is permanently saved at: {}",
                                halvor_config
                            );
                            println!();
                            println!("  To use it, you can:");
                            println!("  1. Merge manually:");
                            println!(
                                "     KUBECONFIG='{}:{}' kubectl config view --flatten > ~/.kube/config",
                                main_config, halvor_config
                            );
                            println!("     kubectl config use-context halvor");
                            println!();
                            println!("  2. Or use it directly:");
                            println!("     export KUBECONFIG='{}'", halvor_config);
                            println!("     kubectl config use-context halvor");

                            println!();
                            println!("  Note: Temp file kept at {} for reference", temp_config);
                        }
                    } else {
                        println!("  ⚠️  Could not execute final merge command");
                        println!(
                            "  The halvor kubeconfig is permanently saved at: {}",
                            halvor_config
                        );
                        println!(
                            "  You can use it directly with: export KUBECONFIG='{}'",
                            halvor_config
                        );
                    }
                } else {
                    merge_success = true;
                    println!("  ✓ Merged kubeconfig using alternative method");
                }
            }
        }

        // Verify the context exists before trying to use it
        let check_context = local::execute_shell("kubectl config get-contexts halvor 2>&1");
        let context_exists = check_context
            .as_ref()
            .map(|r| r.status.success())
            .unwrap_or(false);

        if context_exists {
            // Set halvor as current context
            let context_result = local::execute_shell("kubectl config use-context halvor");
            if let Err(e) = context_result {
                println!("  ⚠️  Warning: Failed to set context as current: {}", e);
                println!("  You can manually set it with: kubectl config use-context halvor");
            } else if let Ok(output) = context_result {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if !stderr.is_empty() {
                        println!("  ⚠️  Warning: {}", stderr.trim());
                    }
                } else {
                    println!("  ✓ Set 'halvor' as current context");
                }
            }
        } else {
            // Context not in main config - check if it's in the temp file
            let check_temp = local::execute_shell(&format!(
                "KUBECONFIG='{}' kubectl config get-contexts halvor 2>&1",
                temp_config
            ));
            let temp_has_context = check_temp
                .as_ref()
                .map(|r| r.status.success())
                .unwrap_or(false);

            if temp_has_context {
                println!("  ⚠️  Context 'halvor' exists in temp file but merge failed");
                println!("  You can use it directly with:");
                println!("      export KUBECONFIG='{}'", temp_config);
                println!("  Or merge manually as shown above");
            } else {
                println!("  ⚠️  Warning: Context 'halvor' not found");
                println!("  The kubeconfig was written to: {}", temp_config);
                println!("  You may need to check the kubeconfig content");
            }
        }

        // Only clean up temp file if merge was successful
        let merge_was_successful = merge_success
            || (std::path::Path::new(&main_config).exists()
                && local::execute_shell("kubectl config get-contexts halvor 2>&1")
                    .as_ref()
                    .map(|r| r.status.success())
                    .unwrap_or(false));

        if merge_was_successful {
            // Clean up temp file only if merge succeeded
            if std::path::Path::new(&temp_config).exists() {
                let _ = std::fs::remove_file(&temp_config);
            }
            println!("  ✓ Kubeconfig set up at {}", main_config);
            println!("  ✓ Context 'halvor' added and set as current");
        } else {
            // Merge failed - keep temp file and inform user
            println!();
            println!("  ⚠️  Note: Merge was not successful, but kubeconfig files are available:");
            if std::path::Path::new(&format!("{}/.kube/halvor.yaml", home)).exists() {
                println!("      Permanent file: ~/.kube/halvor.yaml");
            }
            println!("      Temp file: {}", temp_config);
            println!("  Use the instructions above to merge manually or use the files directly.");
        }

        // Test connection and provide diagnostics
        println!();
        println!("Testing connection (5s timeout)...");
        let test_cmd = if cfg!(unix) {
            "timeout 5 kubectl cluster-info 2>&1 || echo 'TIMEOUT_OR_ERROR'"
        } else {
            "powershell -Command \"$job = Start-Job { kubectl cluster-info 2>&1 }; if (Wait-Job $job -Timeout 5) { Receive-Job $job } else { Stop-Job $job; Remove-Job $job; Write-Host 'TIMEOUT_OR_ERROR' }\""
        };
        let test_result = local::execute_shell(test_cmd);
        if let Ok(output) = test_result {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            if stdout.contains("TIMEOUT_OR_ERROR")
                || (!output.status.success()
                    && stdout.trim().is_empty()
                    && stderr.trim().is_empty())
            {
                println!("⚠ Connection test timed out or failed");
                println!("   This usually means the Kubernetes API server is not reachable.");
                println!(
                    "   Your kubeconfig is configured correctly, but the connection is blocked."
                );
                println!();
                println!("   Common causes:");
                println!("   - Tailscale is not connected or the node is offline");
                println!("   - Firewall blocking port 6443");
                println!("   - Network routing issues");
                println!();
            } else if output.status.success() {
                let info = stdout.to_string();
                println!("{}", info);
                println!("✓ Successfully connected to halvor cluster");
            } else {
                println!("⚠ kubectl configured but connection test failed");
                println!();
                println!("Diagnostics:");
                println!();

                // Check current context
                let context_check = local::execute_shell("kubectl config current-context 2>&1");
                if let Ok(ctx_output) = context_check {
                    let ctx_str = String::from_utf8_lossy(&ctx_output.stdout);
                    let ctx = ctx_str.trim();
                    println!("  Current context: {}", ctx);
                }

                // Check server URL
                let server_check = local::execute_shell(
                    "kubectl config view --minify -o jsonpath='{.clusters[0].cluster.server}' 2>&1",
                );
                if let Ok(server_output) = server_check {
                    let server_str = String::from_utf8_lossy(&server_output.stdout);
                    let server = server_str.trim();
                    println!("  Server URL: {}", server);

                    // Check if server is localhost (problem!)
                    if server.contains("localhost") || server.contains("127.0.0.1") {
                        println!("  ⚠️  PROBLEM: Server URL contains localhost/127.0.0.1");
                        println!("     This won't work from your local machine!");
                        println!(
                            "     The server should be a Tailscale address (e.g., frigg.bombay-pinecone.ts.net:6443)"
                        );
                        println!();
                        println!("  To fix:");
                        println!("    1. Run: halvor config kubeconfig --setup");
                        println!(
                            "    2. Or manually update ~/.kube/config to use Tailscale address"
                        );
                    }
                }

                // Check Tailscale connection
                let ts_check = local::execute_shell("tailscale status --json 2>&1");
                if let Ok(ts_output) = ts_check {
                    if ts_output.status.success() {
                        println!("  Tailscale: Connected");
                    } else {
                        println!("  Tailscale: Not connected or not running");
                        println!("     Run: tailscale status");
                    }
                } else {
                    println!("  Tailscale: Could not check status");
                }

                // Show error details
                if !stderr.trim().is_empty() {
                    println!();
                    println!("  Error details:");
                    println!("  {}", stderr.trim());
                }

                println!();
                println!("  Troubleshooting steps:");
                println!("    1. Verify Tailscale is connected: tailscale status");
                println!("    2. Check kubeconfig server URL: kubectl config view --minify");
                println!("    3. Test connection: kubectl cluster-info");
                println!("    4. Re-run setup: halvor config kubeconfig --setup");
                println!("    5. Run diagnostics: halvor config kubeconfig --diagnose");
            }
        } else {
            println!("⚠ Could not test connection");
        }
    } else {
        // Just print kubeconfig for 1Password
        // First check if halvor context exists
        let context_check = local::execute_shell("kubectl config get-contexts halvor 2>&1");
        let has_halvor_context = context_check
            .as_ref()
            .map(|r| r.status.success())
            .unwrap_or(false);

        if has_halvor_context {
            println!("Current kubeconfig status:");
            println!();

            // Show current context
            let ctx_output = local::execute_shell("kubectl config current-context 2>&1").ok();
            if let Some(ctx) = ctx_output {
                let ctx_name_str = String::from_utf8_lossy(&ctx.stdout);
                let ctx_name = ctx_name_str.trim();
                println!("  Current context: {}", ctx_name);
            }

            // Show server URL
            let server_output = local::execute_shell(
                "kubectl config view --minify -o jsonpath='{.clusters[0].cluster.server}' 2>&1",
            )
            .ok();
            let server_url_opt = server_output.as_ref().and_then(|s| {
                let server_url_str = String::from_utf8_lossy(&s.stdout);
                let trimmed = server_url_str.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            });

            if let Some(server_url) = &server_url_opt {
                println!("  Server URL: {}", server_url);

                // Check if it's localhost (problem!)
                if server_url.contains("localhost") || server_url.contains("127.0.0.1") {
                    println!();
                    println!("  ⚠️  PROBLEM DETECTED: Server URL uses localhost/127.0.0.1");
                    println!("     This won't work from your local machine!");
                    println!("     The server should be a Tailscale address.");
                    println!();
                    println!("  To fix, run:");
                    println!("     halvor config kubeconfig --setup");
                } else if server_url.contains(".ts.net") || server_url.starts_with("100.") {
                    println!("  ✓ Server URL uses Tailscale address (correct)");
                }
            }

            // Test connection with timeout (5 seconds)
            println!();
            println!("  Testing connection (5s timeout)...");
            let test_cmd = if cfg!(unix) {
                "timeout 5 kubectl cluster-info 2>&1 || echo 'TIMEOUT_OR_ERROR'"
            } else {
                "powershell -Command \"$job = Start-Job { kubectl cluster-info 2>&1 }; if (Wait-Job $job -Timeout 5) { Receive-Job $job } else { Stop-Job $job; Remove-Job $job; Write-Host 'TIMEOUT_OR_ERROR' }\""
            };
            let test_output = local::execute_shell(test_cmd).ok();
            if let Some(test) = test_output {
                let stdout = String::from_utf8_lossy(&test.stdout);
                let stderr = String::from_utf8_lossy(&test.stderr);

                if stdout.contains("TIMEOUT_OR_ERROR")
                    || (!test.status.success()
                        && stdout.trim().is_empty()
                        && stderr.trim().is_empty())
                {
                    println!("  ⚠️  Connection test timed out or failed");
                    println!("     This usually means:");
                    println!("     - The Kubernetes API server is not reachable");
                    println!("     - Tailscale connection may be down");
                    println!("     - Firewall blocking port 6443");
                    println!();
                    println!("  Troubleshooting:");
                    println!("    1. Check Tailscale: tailscale status");
                    if let Some(server_url) = &server_url_opt {
                        // Extract hostname/IP from URL (remove https:// and :6443)
                        let host = server_url.replace("https://", "").replace(":6443", "");
                        println!("    2. Test network: ping {}", host);
                        println!("    3. Test port: nc -zv {} 6443", host);
                        println!("    4. Check if K3s is listening on frigg:");
                        println!("       ssh frigg 'sudo netstat -tlnp | grep 6443'");
                        println!("       ssh frigg 'sudo ss -tlnp | grep 6443'");
                        println!("    5. Verify K3s config on frigg:");
                        println!("       ssh frigg 'sudo cat /etc/rancher/k3s/config.yaml'");
                        println!("    6. Check K3s service status on frigg:");
                        println!("       ssh frigg 'sudo systemctl status k3s'");
                    }
                    println!("    7. Verify server URL: kubectl config view --minify");
                    println!("    8. Re-run setup: halvor config kubeconfig --setup");
                } else if test.status.success() {
                    println!("  ✓ Connection successful");
                } else {
                    println!("  ✗ Connection failed");
                    if !stderr.trim().is_empty() {
                        println!("     Error: {}", stderr.trim());
                    } else if !stdout.trim().is_empty() && !stdout.contains("TIMEOUT_OR_ERROR") {
                        println!("     Error: {}", stdout.trim());
                    }
                    println!();
                    println!("  Troubleshooting:");
                    println!("    1. Check Tailscale: tailscale status");
                    println!("    2. Verify server URL: kubectl config view --minify");
                    println!("    3. Re-run setup: halvor config kubeconfig --setup");
                }
            } else {
                println!("  ⚠️  Could not test connection");
            }

            println!();
        }

        // Just print kubeconfig for 1Password
        // Fetch kubeconfig first (this prints status messages)
        let kubeconfig_content = kubeconfig::fetch_kubeconfig_content(&primary_hostname, &config)?;
        println!("{}", kubeconfig_content);
    }

    Ok(())
}

fn handle_regenerate_command(hostname: Option<&str>, yes: bool) -> Result<()> {
    use crate::services::k3s;

    let halvor_dir = find_halvor_dir()?;
    let config = load_env_config(&halvor_dir)?;

    // Default to localhost if not provided
    let target_host = hostname.unwrap_or("localhost");

    k3s::regenerate_certificates(target_host, yes, &config)?;

    Ok(())
}

/// Run comprehensive diagnostics for K3s API server accessibility
pub(crate) fn run_kubeconfig_diagnostics(primary_hostname: &str, config: &EnvConfig) -> Result<()> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("K3s API Server Diagnostics");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Primary node: {}", primary_hostname);
    println!();

    // 1. Check local kubeconfig
    println!("[1/6] Checking local kubeconfig...");
    let context_check = local::execute_shell("kubectl config get-contexts halvor 2>&1");
    let has_halvor_context = context_check
        .as_ref()
        .map(|r| r.status.success())
        .unwrap_or(false);

    if has_halvor_context {
        let server_output = local::execute_shell(
            "kubectl config view --minify -o jsonpath='{.clusters[0].cluster.server}' 2>&1",
        )
        .ok();
        if let Some(server) = server_output {
            let server_str = String::from_utf8_lossy(&server.stdout);
            let server_url = server_str.trim();
            println!("  ✓ Kubeconfig found");
            println!("  Server URL: {}", server_url);

            if server_url.contains("localhost") || server_url.contains("127.0.0.1") {
                println!("  ⚠️  PROBLEM: Server URL uses localhost/127.0.0.1");
            } else if server_url.contains(".ts.net") || server_url.starts_with("100.") {
                println!("  ✓ Server URL uses Tailscale address");
            }
        } else {
            println!("  ⚠️  Could not read server URL from kubeconfig");
        }
    } else {
        println!("  ⚠️  No 'halvor' context found in kubeconfig");
        println!("     Run: halvor config kubeconfig --setup");
    }
    println!();

    // 2. Check Tailscale connectivity
    println!("[2/6] Checking Tailscale connectivity...");
    let ts_status = local::execute_shell("tailscale status --json 2>&1").ok();
    if let Some(status) = ts_status {
        if status.status.success() {
            println!("  ✓ Tailscale is running locally");
            // Try to find primary node in Tailscale status
            let status_str = String::from_utf8_lossy(&status.stdout);
            if status_str.contains(primary_hostname) || status_str.contains(".ts.net") {
                println!("  ✓ Tailscale network is active");
            }
        } else {
            println!("  ⚠️  Tailscale is not running locally");
        }
    } else {
        println!("  ⚠️  Could not check Tailscale status");
    }
    println!();

    // 3. Check if primary node is reachable
    println!("[3/6] Checking primary node reachability...");
    let exec = Executor::new(primary_hostname, config).ok();
    if let Some(exec) = exec {
        if exec.is_local() {
            println!("  ✓ Primary node is local");
        } else {
            println!("  Primary node is remote: {}", primary_hostname);
            // Try to ping
            let ping_result = exec.execute_shell("echo 'ping_test'").ok();
            if ping_result.is_some() {
                println!("  ✓ Can connect to primary node");
            } else {
                println!("  ⚠️  Cannot connect to primary node");
            }
        }

        // Get Tailscale info
        let ts_ip = crate::services::tailscale::get_tailscale_ip_with_fallback(
            &exec,
            primary_hostname,
            config,
        )
        .ok();
        let ts_hostname = crate::services::tailscale::get_tailscale_hostname_remote(&exec)
            .ok()
            .flatten();

        if let Some(ip) = ts_ip {
            println!("  Tailscale IP: {}", ip);
        }
        if let Some(hostname) = ts_hostname {
            println!("  Tailscale hostname: {}", hostname);
        }
    } else {
        println!("  ⚠️  Could not create executor for primary node");
    }
    println!();

    // 4. Check if K3s is running on primary node
    println!("[4/6] Checking K3s service on primary node...");
    let exec_for_k3s = Executor::new(primary_hostname, config).ok();
    if let Some(exec) = exec_for_k3s {
        let k3s_status = exec
            .execute_shell("sudo systemctl is-active k3s 2>&1 || sudo systemctl is-active k3s-agent 2>&1 || echo 'not_running'")
            .ok();
        if let Some(status) = k3s_status {
            let status_str = String::from_utf8_lossy(&status.stdout);
            let status_trimmed = status_str.trim();
            if status_trimmed == "active" {
                println!("  ✓ K3s service is running");
            } else {
                println!(
                    "  ⚠️  K3s service is not running (status: {})",
                    status_trimmed
                );
            }
        }

        // Check if port 6443 is listening
        let port_check = exec
            .execute_shell("sudo netstat -tlnp 2>/dev/null | grep ':6443' || sudo ss -tlnp 2>/dev/null | grep ':6443' || echo 'not_listening'")
            .ok();
        if let Some(port) = port_check {
            let port_str = String::from_utf8_lossy(&port.stdout);
            if port_str.contains("6443") && !port_str.contains("not_listening") {
                println!("  ✓ Port 6443 is listening");
                println!("    {}", port_str.trim());
            } else {
                println!("  ⚠️  Port 6443 is not listening");
            }
        }
    }
    println!();

    // 5. Check K3s configuration
    println!("[5/6] Checking K3s configuration...");
    let exec_for_config = Executor::new(primary_hostname, config).ok();
    if let Some(exec) = exec_for_config {
        let config_check = exec
            .execute_shell("sudo cat /etc/rancher/k3s/config.yaml 2>/dev/null || echo 'no_config'")
            .ok();
        if let Some(config_output) = config_check {
            let config_str = String::from_utf8_lossy(&config_output.stdout);
            if config_str.contains("tls-san") {
                println!("  ✓ K3s config has tls-san entries");
                if config_str.contains(".ts.net") {
                    println!("  ✓ K3s config includes Tailscale hostname");
                }
            } else {
                println!("  ⚠️  K3s config may be missing tls-san entries");
            }
            if config_str.contains("advertise-address") {
                println!("  ✓ K3s config has advertise-address");
            }
        }
    }
    println!();

    // 6. Test API server connectivity
    println!("[6/6] Testing API server connectivity...");
    if has_halvor_context {
        let test_cmd = if cfg!(unix) {
            "timeout 5 kubectl cluster-info 2>&1 || echo 'TIMEOUT_OR_ERROR'"
        } else {
            "powershell -Command \"$job = Start-Job { kubectl cluster-info 2>&1 }; if (Wait-Job $job -Timeout 5) { Receive-Job $job } else { Stop-Job $job; Remove-Job $job; Write-Host 'TIMEOUT_OR_ERROR' }\""
        };
        let test_result = local::execute_shell(test_cmd).ok();
        if let Some(test) = test_result {
            let stdout = String::from_utf8_lossy(&test.stdout);
            if test.status.success() && !stdout.contains("TIMEOUT_OR_ERROR") {
                println!("  ✓ API server is accessible");
                println!("    {}", stdout.lines().next().unwrap_or("").trim());
            } else {
                println!("  ⚠️  API server is not accessible");
                if stdout.contains("TIMEOUT_OR_ERROR") {
                    println!("     Connection timed out");
                } else {
                    let stderr = String::from_utf8_lossy(&test.stderr);
                    if !stderr.trim().is_empty() {
                        println!("     Error: {}", stderr.trim());
                    }
                }
            }
        }
    } else {
        println!("  ⚠️  Skipping (no kubeconfig context)");
    }
    println!();

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("Next steps:");
    println!("  - If kubeconfig is missing: halvor config kubeconfig --setup");
    println!(
        "  - If K3s is not running: ssh {} 'sudo systemctl start k3s'",
        primary_hostname
    );
    println!(
        "  - If port 6443 is not listening: Check K3s service logs on {}",
        primary_hostname
    );
    println!("  - If connection times out: Check firewall rules and Tailscale connectivity");
    println!();

    Ok(())
}

// Backup and commit functions for DB <-> .env synchronization

/// Backup a single host's configuration from database to .env file
pub fn backup_host_config_to_env(hostname: &str) -> Result<()> {
    use crate::config::{env_file, find_halvor_dir};
    use crate::db;

    let halvor_dir = find_halvor_dir()?;
    let env_path = halvor_dir.join(".env");

    // Get host config from database
    let host_config = db::get_host_config(hostname)?
        .ok_or_else(|| anyhow::anyhow!("Host '{}' not found in database", hostname))?;

    // Write to .env file
    env_file::write_host_to_env_file(&env_path, hostname, &host_config)?;

    println!("✓ Backed up host '{}' config to .env", hostname);
    Ok(())
}

/// Backup all hosts' configurations from database to .env file
pub fn backup_all_to_env() -> Result<()> {
    use crate::config::{env_file, find_halvor_dir};
    use crate::db;

    let halvor_dir = find_halvor_dir()?;
    let env_path = halvor_dir.join(".env");

    // Get all hosts from database
    let hosts = db::list_hosts()?;

    if hosts.is_empty() {
        println!("No hosts found in database");
        return Ok(());
    }

    println!("Backing up {} host(s) to .env...", hosts.len());

    for hostname in &hosts {
        if let Ok(Some(host_config)) = db::get_host_config(hostname) {
            env_file::write_host_to_env_file(&env_path, hostname, &host_config)?;
            println!("  ✓ {}", hostname);
        }
    }

    println!("✓ Backed up all hosts to .env");
    Ok(())
}

/// Commit a single host's configuration from .env file to database
pub fn commit_host_config_to_db(hostname: &str) -> Result<()> {
    use crate::config::{find_halvor_dir, load_env_config};
    use crate::db;

    let halvor_dir = find_halvor_dir()?;
    let config = load_env_config(&halvor_dir)?;

    // Get host config from .env
    let host_config = config
        .hosts
        .get(hostname)
        .ok_or_else(|| anyhow::anyhow!("Host '{}' not found in .env file", hostname))?
        .clone();

    // Store in database
    db::store_host_config(hostname, &host_config)?;

    println!("✓ Committed host '{}' config to database", hostname);
    Ok(())
}

/// Commit all hosts' configurations from .env file to database
pub fn commit_all_to_db() -> Result<()> {
    use crate::config::{find_halvor_dir, load_env_config};
    use crate::db;

    let halvor_dir = find_halvor_dir()?;
    let config = load_env_config(&halvor_dir)?;

    if config.hosts.is_empty() {
        println!("No hosts found in .env file");
        return Ok(());
    }

    println!("Committing {} host(s) to database...", config.hosts.len());

    for (hostname, host_config) in &config.hosts {
        db::store_host_config(hostname, host_config)?;
        println!("  ✓ {}", hostname);
    }

    println!("✓ Committed all hosts to database");
    Ok(())
}

/// Backup the SQLite database to a file
pub fn backup_database(path: Option<&str>) -> Result<()> {
    use crate::db;
    use std::fs;
    use std::path::PathBuf;

    let db_path = db::get_db_path()?;

    // Determine backup path
    let backup_path = if let Some(p) = path {
        PathBuf::from(p)
    } else {
        // Default: current directory with timestamp
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        std::env::current_dir()?.join(format!("halvor-db-backup-{}.db", timestamp))
    };

    // Check if source database exists
    if !db_path.exists() {
        anyhow::bail!("Database not found at: {}", db_path.display());
    }

    println!("Backing up database...");
    println!("  From: {}", db_path.display());
    println!("  To:   {}", backup_path.display());

    // Copy the database file
    fs::copy(&db_path, &backup_path).with_context(|| {
        format!(
            "Failed to copy database from {} to {}",
            db_path.display(),
            backup_path.display()
        )
    })?;

    println!("✓ Database backed up successfully");
    Ok(())
}
