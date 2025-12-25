//! K3s cluster initialization

use crate::config::EnvConfig;
use crate::services::k3s::utils::{generate_cluster_token, parse_node_token};
use crate::services::k3s::{agent_service, cleanup, tools};
use crate::services::tailscale;
use crate::utils::exec::{CommandExecutor, Executor};
use anyhow::{Context, Result};
use std::io::{self, Write};

/// Check if an existing K3s cluster is running and return its full token if found
/// Returns the full node-token format (K<node-id>::server:<token>)
fn check_existing_cluster<E: CommandExecutor>(exec: &E) -> Result<Option<String>> {
    // Check if K3s service is running - use sudo since systemctl requires it
    // Use tee to capture output while showing it
    let status_tmp = format!("/tmp/halvor_k3s_status_check_{}", std::process::id());
    use crate::utils::ssh::shell_escape;
    let escaped_tmp = shell_escape(&status_tmp);
    let status_cmd = format!("sudo systemctl is-active k3s 2>&1 | tee {} || sudo systemctl is-active k3s-agent 2>&1 | tee {} || echo 'inactive' | tee {}", escaped_tmp, escaped_tmp, escaped_tmp);
    
    let _ = exec.execute_shell_interactive(&status_cmd);
    
    let k3s_running = exec
        .read_file(&status_tmp)
        .ok()
        .map(|s| {
            let trimmed = s.trim();
            trimmed == "active" || trimmed == "activating"
        })
        .unwrap_or(false);
    
    // Clean up temp file
    let _ = exec.execute_shell(&format!("rm -f {}", shell_escape(&status_tmp)));

    if !k3s_running {
        return Ok(None);
    }

    // Check if this is a server node (has node-token file)
    // Use sudo to check since the file is owned by root
    let token_check_tmp = format!("/tmp/halvor_k3s_token_check_{}", std::process::id());
    let escaped_token_tmp = shell_escape(&token_check_tmp);
    let token_check_cmd = format!("sudo test -f /var/lib/rancher/k3s/server/node-token 2>&1 && echo 'exists' | tee {} || echo 'not_exists' | tee {}", escaped_token_tmp, escaped_token_tmp);
    
    let _ = exec.execute_shell_interactive(&token_check_cmd);
    
    let has_node_token = exec
        .read_file(&token_check_tmp)
        .ok()
        .map(|s| s.trim() == "exists")
        .unwrap_or(false);
    
    // Clean up temp file
    let _ = exec.execute_shell(&format!("rm -f {}", escaped_token_tmp));

    if !has_node_token {
        // K3s is running but this might be an agent node, not a server
        return Ok(None);
    }

    // Try to read the current token (write to temp file first, then read it)
    let token_tmp = "/tmp/k3s_cluster_token_check";
    let token_result = exec.execute_shell_interactive(&format!(
        "sudo cat /var/lib/rancher/k3s/server/node-token > {} 2>/dev/null || echo '' > {}",
        token_tmp, token_tmp
    ));

    if token_result.is_ok() {
        if let Ok(token_content) = exec.read_file(token_tmp) {
            let trimmed = token_content.trim().to_string();
            if !trimmed.is_empty() && trimmed != "''" {
                // Clean up temp file
                let _ = exec.execute_shell(&format!("rm -f {}", token_tmp));
                // Return the full token format (not parsed)
                return Ok(Some(trimmed));
            }
        }
    }

    // Clean up temp file
    let _ = exec.execute_shell(&format!("rm -f {}", token_tmp));

    // If we can't read the token but cluster exists, still warn
    Ok(Some("<token unavailable>".to_string()))
}

/// Initialize the first control plane node with embedded etcd
pub fn init_control_plane(
    hostname: &str,
    token: Option<&str>,
    yes: bool,
    config: &EnvConfig,
) -> Result<()> {
    let exec = Executor::new(hostname, config)?;
    let is_local = exec.is_local();

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Initialize K3s HA Cluster - First Control Plane Node");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    if is_local {
        println!("Target: localhost ({})", hostname);
    } else {
        println!("Target: {} (remote)", hostname);
    }
    println!();

    // Check for existing K3s cluster BEFORE generating token or prompting
    println!("Checking for existing K3s cluster...");
    let existing_cluster = check_existing_cluster(&exec)?;
    let had_existing_cluster = existing_cluster.is_some();
    
    if let Some(ref existing_token) = existing_cluster {
        println!();
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("⚠️  WARNING: A K3s cluster already exists on this node!");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!();
        println!("Current cluster token:");
        println!("  {}", existing_token);
        println!();
        println!("⚠️  Initializing a new cluster will:");
        println!("   - Remove the existing K3s installation");
        println!("   - Delete all cluster data and workloads");
        println!("   - Require all nodes to rejoin with the new token");
        println!();
        
        if yes {
            println!("⚠️  --yes flag is set, proceeding with overwrite...");
            println!();
        } else {
            print!("Do you want to overwrite the existing cluster? [y/N]: ");
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            if !input.trim().eq_ignore_ascii_case("y") {
                println!("Aborted. Existing cluster will not be modified.");
                println!();
                // Get Tailscale info for join command
                let tailscale_ip = tailscale::get_tailscale_ip_with_fallback(&exec, hostname, config)
                    .unwrap_or_else(|_| hostname.to_string());
                let tailscale_hostname = tailscale::get_tailscale_hostname_remote(&exec)
                    .ok()
                    .flatten();
                println!("To join this existing cluster, use:");
                // Parse token for use in command (K3s accepts both full format and token-only)
                let token_for_join = parse_node_token(existing_token);
                println!("  halvor join <hostname> --server={} --token={}", 
                    tailscale_hostname.as_ref().unwrap_or(&tailscale_ip),
                    token_for_join);
                println!();
                
                // Still set up halvor agent service even if not initializing cluster
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                println!("Setting up halvor agent service");
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                println!();
                
                let web_port = if std::env::var("HALVOR_WEB_DIR").is_ok() {
                    Some(13000)
                } else {
                    None
                };
                
                if let Err(e) = agent_service::setup_agent_service(&exec, web_port) {
                    eprintln!("⚠️  Warning: Failed to setup halvor agent service: {}", e);
                    eprintln!("   You can set it up manually later with: halvor agent start --port 13500 --daemon");
                } else {
                    println!("✓ Halvor agent service is running on {}", hostname);
                    println!("  Agent API: port 13500 (over Tailscale)");
                    if web_port.is_some() {
                        println!("  Web UI: port 13000 (over Tailscale)");
                    }
                }
                println!();
                
                return Ok(());
            }
            println!();
        }
    } else {
        println!("✓ No existing cluster detected");
        println!();
    }

    // Generate token if not provided
    let cluster_token = if let Some(t) = token {
        let trimmed = t.trim();
        if trimmed.is_empty() {
            anyhow::bail!("Token cannot be empty. A token was provided but it was empty.");
        }
        println!("Using provided cluster token: {}", trimmed);
        trimmed.to_string()
    } else {
        println!("Generating cluster token...");
        let generated_token = generate_cluster_token()
            .context("Failed to generate cluster token using any available method")?;
        println!("Generated cluster token: {}", generated_token);
        generated_token
    };

    // Verify token is not empty before proceeding
    if cluster_token.is_empty() {
        anyhow::bail!(
            "Cluster token is empty after generation/assignment. This should not happen."
        );
    }

    println!("Cluster token: {}", cluster_token);
    println!();

    if !yes && !had_existing_cluster {
        print!("This will initialize a new K3s HA cluster. Continue? [y/N]: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            println!();
            
            // Still set up halvor agent service even if not initializing cluster
            // (but only if it's not already set up)
            let service_exists = exec.file_exists("/etc/systemd/system/halvor-agent.service").unwrap_or(false);
            let service_active = exec
                .execute_shell("systemctl is-active halvor-agent.service 2>/dev/null || echo inactive")
                .ok()
                .and_then(|o| {
                    String::from_utf8(o.stdout).ok().map(|s| s.trim() == "active")
                })
                .unwrap_or(false);

            if service_exists && service_active {
                println!("✓ Halvor agent service is already running");
                println!();
            } else {
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                println!("Setting up halvor agent service");
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                println!();
                
                let web_port = if std::env::var("HALVOR_WEB_DIR").is_ok() {
                    Some(13000)
                } else {
                    None
                };
                
                if let Err(e) = agent_service::setup_agent_service(&exec, web_port) {
                    eprintln!("⚠️  Warning: Failed to setup halvor agent service: {}", e);
                    eprintln!("   You can set it up manually later with: halvor agent start --port 13500 --daemon");
                } else {
                    println!("✓ Halvor agent service is running on {}", hostname);
                    println!("  Agent API: port 13500 (over Tailscale)");
                    if web_port.is_some() {
                        println!("  Web UI: port 13000 (over Tailscale)");
                    }
                }
                println!();
            }
            
            return Ok(());
        }
    }

    // Get Tailscale IP and hostname for cluster communication
    println!("Getting Tailscale IP for cluster communication...");
    let tailscale_ip = tailscale::get_tailscale_ip_with_fallback(&exec, hostname, config)?;

    let tailscale_hostname = tailscale::get_tailscale_hostname_remote(&exec)
        .ok()
        .flatten();

    println!("✓ Using Tailscale IP: {}", tailscale_ip);
    if let Some(ref ts_hostname) = tailscale_hostname {
        println!("✓ Using Tailscale hostname: {}", ts_hostname);
    }

    // Check for existing K3s installation and clean it up if found
    // Skip prompt if we already confirmed overwrite above
    println!("Checking for existing K3s installation...");
    let skip_cleanup_prompt = had_existing_cluster && yes;
    cleanup::cleanup_existing_k3s_with_prompt(&exec, !skip_cleanup_prompt)?;

    // Ensure Tailscale is installed and running (required for cluster communication)
    println!();
    println!("Checking for Tailscale (required for cluster communication)...");
    if !tailscale::is_tailscale_installed(&exec) {
        println!("Tailscale not found. Installing Tailscale...");
        if hostname == "localhost" {
            tailscale::install_tailscale()?;
        } else {
            tailscale::install_tailscale_on_host(hostname, config)?;
        }
    } else {
        println!("✓ Tailscale is installed");
    }
    
    // Check if Tailscale is running and connected
    let tailscale_status = exec
        .execute_shell("tailscale status --json 2>/dev/null || echo 'not_running'")
        .ok();
    
    let is_tailscale_running = tailscale_status
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| !s.contains("not_running") && !s.trim().is_empty())
        .unwrap_or(false);
    
    if !is_tailscale_running {
        println!("⚠️  Warning: Tailscale may not be running or connected.");
        println!("   Please ensure Tailscale is running and authenticated before continuing.");
        println!("   Run: sudo tailscale up");
        if !yes {
            print!("Continue anyway? [y/N]: ");
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            if !input.trim().eq_ignore_ascii_case("y") {
                println!("Aborted.");
                return Ok(());
            }
        }
    } else {
        println!("✓ Tailscale is running");
    }

    // Ensure halvor is installed first (the glue that enables remote operations)
    println!();
    println!("Checking for halvor (required for remote operations)...");
    tools::check_and_install_halvor(&exec)?;

    // Ensure kubectl and helm are installed
    println!();
    println!("Checking for required tools...");
    tools::check_and_install_kubectl(&exec)?;
    tools::check_and_install_helm(&exec)?;

    // Note: SMB mounts are set up separately for cluster storage, not for K3s data directory
    // K3s will use default local data directory (/var/lib/rancher/k3s)
    // SMB mounts will be available in the cluster for persistent volumes

    println!();
    println!("Installing K3s with embedded etcd...");

    // Build TLS SANs list (include Tailscale IP and hostname)
    let mut tls_sans = format!("--tls-san={}", tailscale_ip);
    if let Some(ref ts_hostname) = tailscale_hostname {
        tls_sans.push_str(&format!(" --tls-san={}", ts_hostname));
    }
    tls_sans.push_str(&format!(" --tls-san={}", hostname));

    // Download K3s install script using reqwest
    println!("Downloading K3s install script...");
    let k3s_script_url = "https://get.k3s.io";
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("Failed to create HTTP client")?;

    let k3s_script = client
        .get(k3s_script_url)
        .send()
        .context("Failed to download K3s install script")?
        .error_for_status()
        .context("HTTP error downloading K3s install script")?
        .text()
        .context("Failed to read K3s install script content")?;

    // Detect init system to help K3s script detect correctly
    println!("Detecting init system...");
    let has_systemd = exec
        .execute_shell("systemctl --version >/dev/null 2>&1 && echo yes || echo no")
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim() == "yes")
        .unwrap_or(false);
    
    let has_openrc = exec
        .execute_shell("command -v rc-update >/dev/null 2>&1 && echo yes || echo no")
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim() == "yes")
        .unwrap_or(false);
    
    if has_systemd {
        println!("✓ Detected systemd init system");
    } else if has_openrc {
        println!("✓ Detected OpenRC init system");
    } else {
        println!("⚠ Warning: Could not detect init system, assuming systemd");
    }
    
    // Patch the K3s script to fix incorrect init system detection and sudo issues
    // The script incorrectly tries to remove /etc/systemd/system when it detects OpenRC
    // We'll prevent that by ensuring systemd detection is correct
    let mut patched_script = k3s_script;
    
    // Fix the dangerous rm -f /etc/systemd/system line that appears when detection is wrong
    // Replace it with a safer check
    if patched_script.contains("rm -f /etc/systemd/system") {
        println!("⚠ Patching K3s script to fix incorrect init system detection...");
        patched_script = patched_script.replace(
            "rm -f /etc/systemd/system",
            "# Patched by halvor: removed dangerous rm command"
        );
    }
    
    // Fix broken sudo calls in the K3s script
    // The script sometimes calls sudo incorrectly, causing "usage: sudo" errors
    println!("⚠ Patching K3s script to fix sudo handling...");
    
    // Fix common sudo issues in the script:
    // 1. Replace "sudo " with proper sudo calls (only if followed by a command)
    // 2. Fix cases where sudo might be called with no arguments
    // 3. Ensure sudo commands are properly formatted
    
    // Fix pattern: sudo followed by just whitespace or newline (invalid)
    patched_script = patched_script.replace("sudo \n", "sudo -v\n");
    patched_script = patched_script.replace("sudo \r\n", "sudo -v\r\n");
    
    // Fix pattern: sudo$ (sudo at end of line with no command)
    patched_script = patched_script.replace("sudo$\n", "sudo -v\n");
    
    // Fix pattern: sudo || (sudo with no command before ||)
    patched_script = patched_script.replace("sudo ||", "sudo -v ||");
    
    // Fix pattern: sudo && (sudo with no command before &&)
    patched_script = patched_script.replace("sudo &&", "sudo -v &&");
    
    // Add helper functions at the beginning to ensure sudo works and disable pagers
    let helper_fixes = r#"
       # Patched by halvor: Ensure sudo works correctly
       _sudo() {
           if [ "$(id -u)" = "0" ]; then
               # Already root, no sudo needed
               "$@"
           else
               # Not root, use sudo
               if command -v sudo >/dev/null 2>&1; then
                   sudo "$@"
               else
                   echo "ERROR: sudo is required but not found" >&2
                   exit 1
               fi
           fi
       }

       # Patched by halvor: Create a wrapper script in a temp directory that's early in PATH
       # This catches ALL systemctl calls, including those in subshells
       _HALVOR_TMP_BIN="$(mktemp -d)"
       export PATH="$_HALVOR_TMP_BIN:$PATH"
       # Use a single printf with escaped newlines for sh compatibility
       printf '#!/bin/sh\n# Halvor wrapper for systemctl to disable pager\n_cmd="$1"\nshift\ncase "$_cmd" in\n    list-units|list-unit-files|status|show|cat)\n        _has_no_pager=false\n        for _arg in "$@"; do\n            if [ "$_arg" = "--no-pager" ] || [ "$_arg" = "-n" ]; then\n                _has_no_pager=true\n                break\n            fi\n        done\n        if [ "$_has_no_pager" = false ]; then\n            exec /usr/bin/systemctl "$_cmd" --no-pager "$@"\n        else\n            exec /usr/bin/systemctl "$_cmd" "$@"\n        fi\n        ;;\n    *)\n        exec /usr/bin/systemctl "$_cmd" "$@"\n        ;;\nesac\n' > "$_HALVOR_TMP_BIN/systemctl"
       chmod +x "$_HALVOR_TMP_BIN/systemctl"
       
       # Also create a bash function wrapper for better compatibility (if bash is available)
       if command -v bash >/dev/null 2>&1; then
           _systemctl() {
               local cmd="$1"
               shift
               local needs_no_pager=true
               case "$cmd" in
                   list-units|list-unit-files|status|show|cat)
                       for arg in "$@"; do
                           if [ "$arg" = "--no-pager" ] || [ "$arg" = "-n" ]; then
                               needs_no_pager=false
                               break
                           fi
                       done
                       if [ "$needs_no_pager" = true ]; then
                           command systemctl "$cmd" --no-pager "$@"
                       else
                           command systemctl "$cmd" "$@"
                       fi
                       ;;
                   *)
                       command systemctl "$cmd" "$@"
                       ;;
               esac
           }
           export -f _systemctl 2>/dev/null || true
           alias systemctl='_systemctl' 2>/dev/null || true
       fi
"#;
    
    // Insert the fixes near the beginning of the script (after shebang)
    if let Some(pos) = patched_script.find('\n') {
        let (shebang, rest) = patched_script.split_at(pos + 1);
        patched_script = format!("{}{}{}", shebang, helper_fixes, rest);
        
        // Replace systemctl calls with _systemctl wrapper (but preserve sudo if present)
        // Pattern: systemctl <command> -> _systemctl <command>
        // Pattern: sudo systemctl <command> -> _sudo _systemctl <command>
        // Do this in multiple passes to catch all cases
        
        // First, handle sudo systemctl (must come before plain systemctl)
        patched_script = patched_script.replace("sudo systemctl ", "_sudo _systemctl ");
        
        // Then handle plain systemctl in various contexts
        // Replace at start of line
        patched_script = patched_script.replace("\nsystemctl ", "\n_systemctl ");
        // Replace after tab
        patched_script = patched_script.replace("\tsystemctl ", "\t_systemctl ");
        // Replace after space (but be careful not to break strings)
        // Only replace if followed by a command word (not in quotes)
        patched_script = patched_script.replace(" systemctl ", " _systemctl ");
        // Replace in backticks
        patched_script = patched_script.replace("`systemctl ", "`_systemctl ");
        // Replace in command substitution
        patched_script = patched_script.replace("$(systemctl ", "$(_systemctl ");
        // Replace at start of command (after && or ||)
        patched_script = patched_script.replace("&& systemctl ", "&& _systemctl ");
        patched_script = patched_script.replace("|| systemctl ", "|| _systemctl ");
        // Replace after semicolon
        patched_script = patched_script.replace("; systemctl ", "; _systemctl ");
        
        // Replace common sudo patterns with _sudo helper
        patched_script = patched_script.replace("sudo mkdir", "_sudo mkdir");
        patched_script = patched_script.replace("sudo tee", "_sudo tee");
        patched_script = patched_script.replace("sudo chmod", "_sudo chmod");
        patched_script = patched_script.replace("sudo chown", "_sudo chown");
        
        // Add cleanup of temp bin directory at the end of the script
        // Find the last line and add cleanup before it
        if let Some(last_newline) = patched_script.rfind('\n') {
            let (before, after) = patched_script.split_at(last_newline);
            patched_script = format!("{}\n# Cleanup halvor temp bin directory\nrm -rf \"$_HALVOR_TMP_BIN\" 2>/dev/null || true\n{}", before, after);
        }
    }
    
    // If systemd is detected, ensure the script knows about it
    if has_systemd && !has_openrc {
        // Set environment variable to help script detection
        // The script checks for systemd by looking for systemctl
        // We've already verified it exists, so the script should detect it correctly
    }

    // Write patched script to remote host
    let remote_script_path = "/tmp/k3s-install.sh";
    exec.write_file(remote_script_path, patched_script.as_bytes())
        .context("Failed to write K3s install script to remote host")?;

    // Make script executable using execute_simple (same approach as Helm)
    let chmod_output = exec.execute_simple("chmod", &["+x", remote_script_path])?;
    if !chmod_output.status.success() {
        anyhow::bail!(
            "Failed to make K3s install script executable: {}",
            String::from_utf8_lossy(&chmod_output.stderr)
        );
    }

    // Build the install command with Tailscale IP in TLS SANs and as advertise address
    // Use Tailscale IP for cluster communication
    let advertise_addr = format!("--advertise-address={}", tailscale_ip);
    
    // The K3s script handles sudo internally, but we'll run it with sudo from the start if we have the password
    // This avoids issues with the script's internal sudo detection and re-execution
    // Environment variables to disable pagers are set automatically by execute_shell_interactive
    let install_cmd = if exec.get_username().ok().as_deref() == Some("root") {
        // Already running as root, no sudo needed
        format!(
            "{} server --cluster-init --token={} --disable=traefik --etcd-expose-metrics --write-kubeconfig-mode=0644 {} {}",
            remote_script_path, cluster_token, advertise_addr, tls_sans
        )
    } else {
        // Not root - run with sudo to avoid script's internal sudo handling issues
        // Use -E flag to preserve environment variables (PAGER, SYSTEMD_PAGER, etc.)
        format!(
            "sudo -E {} server --cluster-init --token={} --disable=traefik --etcd-expose-metrics --write-kubeconfig-mode=0644 {} {}",
            remote_script_path, cluster_token, advertise_addr, tls_sans
        )
    };

    // Execute the install command with output capture for error analysis
    // If we have sudo password, it will be injected automatically by execute_shell_interactive
    // The environment variables ensure no pagers are used and commands are non-interactive
    let install_output_file = format!("/tmp/k3s_install_output_{}", std::process::id());
    let install_cmd_with_capture = format!("{} 2>&1 | tee {}", install_cmd, install_output_file);
    
    println!("Executing K3s installation...");
    println!("Install command: {}", install_cmd);
    println!();
    
    let install_result = exec.execute_shell_interactive(&install_cmd_with_capture);
    
    // Read the captured output to see what actually happened
    let install_output = exec.read_file(&install_output_file).ok().unwrap_or_default();
    let _ = exec.execute_shell(&format!("rm -f {}", install_output_file));
    
    // Even if the install script exits with an error, check if the service was created
    // Sometimes the script exits with an error but the service is still set up
    let service_exists = exec
        .execute_shell("systemctl list-unit-files --no-pager 2>/dev/null | grep -q '^k3s.service' && echo exists || echo not_exists")
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim() == "exists")
        .unwrap_or(false);
    
    if let Err(e) = install_result {
        // If service exists, it might have been created despite the error
        // Wait a bit and check if it's actually running
        if service_exists {
            println!("⚠️  Install script reported an error, but service was created.");
            println!("   Waiting to see if service starts successfully...");
            std::thread::sleep(std::time::Duration::from_secs(15));
            
            // Check if service is actually running now
            let is_running = exec
                .execute_shell("systemctl is-active k3s 2>/dev/null || echo inactive")
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim() == "active")
                .unwrap_or(false);
            
            if is_running {
                println!("✓ K3s service is running despite install script error");
                // Continue with verification below
            } else {
                // Service exists but isn't running - show diagnostics
                println!();
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                println!("K3s installation failed. Install script output:");
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                if !install_output.is_empty() {
                    println!("{}", install_output);
                } else {
                    println!("(No output captured)");
                }
                println!();
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                println!("Checking service status and logs...");
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                println!();
                
                let status_output = exec.execute_shell("sudo systemctl status k3s --no-pager -n 30 2>&1 || echo 'Service not found'").ok();
                if let Some(status) = status_output {
                    let status_str = String::from_utf8_lossy(&status.stdout);
                    if !status_str.trim().is_empty() && !status_str.contains("Service not found") {
                        println!("Service status:");
                        println!("{}", status_str);
                        println!();
                    }
                }
                
                let journal_output = exec.execute_shell("sudo journalctl -u k3s.service --no-pager -n 50 2>&1 || echo 'Unable to get logs'").ok();
                if let Some(journal) = journal_output {
                    let journal_str = String::from_utf8_lossy(&journal.stdout);
                    if !journal_str.trim().is_empty() && !journal_str.contains("Unable to get logs") {
                        println!("Recent service logs:");
                        println!("{}", journal_str);
                        println!();
                    }
                }
                
                // Kill leftover containerd processes that might be blocking startup
                println!("Attempting to clean up leftover processes...");
                let _ = exec.execute_shell("sudo pkill -9 containerd-shim 2>/dev/null || true");
                let _ = exec.execute_shell("sudo systemctl reset-failed k3s.service 2>/dev/null || true");
                std::thread::sleep(std::time::Duration::from_secs(2));
                let _ = exec.execute_shell("sudo systemctl start k3s.service 2>&1 || true");
                
                anyhow::bail!(
                    "Failed to install K3s: {}\n\
                     \n\
                     Check the service status and logs above for details.\n\
                     Common issues:\n\
                     - Leftover containerd processes blocking startup\n\
                     - Data directory permissions\n\
                     - Network configuration\n\
                     \n\
                     To investigate further:\n\
                       sudo journalctl -u k3s.service -n 100 --no-pager\n\
                       sudo systemctl status k3s.service --no-pager",
                    e
                );
            }
        } else {
            // Service doesn't exist, installation definitely failed
            println!();
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("K3s installation failed. Install script output:");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            if !install_output.is_empty() {
                println!("{}", install_output);
            } else {
                println!("(No output captured)");
            }
            println!();
            
            anyhow::bail!(
                "Failed to install K3s: {}\n\
                 \n\
                 The K3s service was not created. Check the install script output above for details.",
                e
            );
        }
    }

    // Clean up script
    let _ = exec.execute_shell(&format!("rm -f {}", remote_script_path));

    println!();
    println!("✓ K3s installation command completed");

    // Configure K3s service to depend on Tailscale
    println!("Configuring K3s service to depend on Tailscale...");
    let service_override_dir = "/etc/systemd/system/k3s.service.d";
    let override_content = r#"[Unit]
After=tailscale.service
Wants=tailscale.service
Requires=network-online.target
"#;
    
    // Create override directory
    let _ = exec.execute_shell(&format!("sudo mkdir -p {}", service_override_dir));
    
    // Write override file
    let override_file = format!("{}/tailscale.conf", service_override_dir);
    if let Err(e) = exec.write_file(&override_file, override_content.as_bytes()) {
        println!("⚠️  Warning: Failed to create systemd override: {}", e);
        println!("   K3s service may start before Tailscale is ready.");
    } else {
        println!("✓ Created systemd override to ensure K3s starts after Tailscale");
        
        // Reload systemd to pick up the override
        let _ = exec.execute_shell("sudo systemctl daemon-reload");
    }

    // Wait for service to start and verify it's running
    println!("Waiting for K3s service to start...");
    std::thread::sleep(std::time::Duration::from_secs(10));

    // Verify K3s service is running
    println!("Verifying K3s service is running...");
    for attempt in 1..=6 {
        // Use execute_simple to check service status directly (more reliable than temp files)
        // Note: systemctl is-active doesn't require sudo, but may need it in some cases
        let status_output = exec.execute_simple("systemctl", &["is-active", "k3s"]).ok();

        let is_active = if let Some(output) = &status_output {
            let status_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let success = output.status.success();
            // systemctl is-active returns "active" if running, non-zero exit if not
            success && status_str == "active"
        } else {
            false
        };

        if is_active {
            println!("✓ K3s service is running");
            break;
        } else if attempt < 6 {
            println!(
                "  Service not active yet, waiting... (attempt {}/6)",
                attempt
            );
            std::thread::sleep(std::time::Duration::from_secs(5));
        } else {
            // Get more details about why it's not running
            // Get detailed error information
            let status_details = exec.execute_shell("sudo systemctl status k3s --no-pager -n 30 2>&1 || echo 'Unable to get status'").ok();
            let details = status_details
                .map(|c| String::from_utf8_lossy(&c.stdout).trim().to_string())
                .unwrap_or_else(|| "Unable to get service status".to_string());
            
            // Also get journal logs for more details
            let journal_logs = exec.execute_shell("sudo journalctl -u k3s.service --no-pager -n 50 2>&1 || echo 'Unable to get logs'").ok();
            let logs = journal_logs
                .map(|c| String::from_utf8_lossy(&c.stdout).trim().to_string())
                .unwrap_or_else(|| "Unable to get journal logs".to_string());

            anyhow::bail!(
                "K3s service failed to start after {} attempts.\n\
                 \n\
                 Service status:\n{}\n\
                 \n\
                 Recent logs:\n{}\n\
                 \n\
                 To investigate further:\n\
                   sudo journalctl -u k3s.service -n 100 --no-pager",
                attempt,
                details,
                logs
            );
        }
    }

    // Wait for kubeconfig to be generated and API server to be ready
    println!("Waiting for kubeconfig and API server to be ready...");
    println!("  (This may take 1-2 minutes for embedded etcd to initialize)");

    let max_wait_attempts = 24; // 24 * 5 seconds = 2 minutes max
    let mut api_ready = false;

    for attempt in 1..=max_wait_attempts {
        // Check if kubeconfig exists and API server is responding
        let kubeconfig_check = exec
            .execute_shell(
                "sudo test -f /etc/rancher/k3s/k3s.yaml && echo 'exists' || echo 'missing'",
            )
            .ok();
        let kubeconfig_exists = kubeconfig_check
            .map(|c| String::from_utf8_lossy(&c.stdout).trim() == "exists")
            .unwrap_or(false);

        if kubeconfig_exists {
            // Try to query the API server to see if it's ready
            let api_check = exec
                .execute_shell("sudo k3s kubectl get nodes --request-timeout=5s 2>&1 | head -1")
                .ok();
            if let Some(api_output) = api_check {
                let api_str = String::from_utf8_lossy(&api_output.stdout);
                // If we get a response (even if it's empty or an error), API server is likely up
                // Check for common "not ready" errors
                if !api_str.contains("Unable to connect")
                    && !api_str.contains("connection refused")
                    && !api_str.contains("dial tcp")
                    && api_output.status.code() != Some(1)
                {
                    api_ready = true;
                    break;
                }
            }
        }

        if attempt < max_wait_attempts {
            if attempt % 6 == 0 {
                // Print progress every 30 seconds
                println!("  Still waiting... ({} seconds elapsed)", attempt * 5);
            }
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    }

    if !api_ready {
        println!("⚠ Warning: API server may not be fully ready yet.");
        println!("  The cluster is still initializing. You can check status with:");
        println!("  halvor status k3s -H {}", hostname);
        println!("  Or wait a few more minutes and try joining nodes.");
    } else {
        println!("✓ API server is ready");
    }

    println!();
    println!("✓ K3s initialized successfully!");
    println!();

    // Ensure token is not empty before printing
    if cluster_token.is_empty() {
        anyhow::bail!(
            "Cluster token is empty - this should not happen. Token generation may have failed."
        );
    }

    // Get and print kubeconfig for 1Password
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Kubeconfig (add to 1Password as KUBE_CONFIG environment variable):");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    
    // Read kubeconfig from the server
    let kubeconfig_output = exec
        .execute_shell("sudo cat /etc/rancher/k3s/k3s.yaml 2>/dev/null")
        .ok();
    
    if let Some(output) = kubeconfig_output {
        if output.status.success() {
            let mut kubeconfig_content = String::from_utf8_lossy(&output.stdout).to_string();
            
            // Replace localhost/127.0.0.1 with Tailscale address
            let server_addr = tailscale_hostname.as_ref().unwrap_or(&tailscale_ip);
            kubeconfig_content = kubeconfig_content.replace("127.0.0.1", server_addr);
            kubeconfig_content = kubeconfig_content.replace("localhost", server_addr);
            
            // Print the kubeconfig
            println!("{}", kubeconfig_content);
            println!();
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("⚠️  IMPORTANT: Copy the kubeconfig above and add it to your 1Password vault");
            println!("   as the KUBE_CONFIG environment variable.");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!();
        } else {
            println!("⚠️  Warning: Could not read kubeconfig file. You may need to retrieve it manually:");
            println!("   ssh {} 'sudo cat /etc/rancher/k3s/k3s.yaml' | sed 's|127.0.0.1|{}|g' | sed 's|localhost|{}|g'", 
                hostname, 
                tailscale_hostname.as_ref().unwrap_or(&tailscale_ip),
                tailscale_hostname.as_ref().unwrap_or(&tailscale_ip));
            println!();
        }
    } else {
        println!("⚠️  Warning: Could not read kubeconfig file. You may need to retrieve it manually:");
        println!("   ssh {} 'sudo cat /etc/rancher/k3s/k3s.yaml' | sed 's|127.0.0.1|{}|g' | sed 's|localhost|{}|g'", 
            hostname, 
            tailscale_hostname.as_ref().unwrap_or(&tailscale_ip),
            tailscale_hostname.as_ref().unwrap_or(&tailscale_ip));
        println!();
    }

    println!("Save this token to join additional nodes:");
    println!("  K3S_TOKEN={}", cluster_token);
    println!();
    println!("  Note: For 1Password, store only the token part (after ::server: if present)");
    println!();
    println!("Join additional control plane nodes with:");
    let server_addr = tailscale_hostname.as_ref().unwrap_or(&tailscale_ip);
    println!(
        "  halvor join <hostname> --server={} --token={}",
        server_addr, cluster_token
    );
    println!();

    // Setup halvor agent service on the primary node (only if not already set up)
    let exec = Executor::new(hostname, config)?;
    let service_exists = exec.file_exists("/etc/systemd/system/halvor-agent.service").unwrap_or(false);
    let service_active = exec
        .execute_shell("systemctl is-active halvor-agent.service 2>/dev/null || echo inactive")
        .ok()
        .and_then(|o| {
            String::from_utf8(o.stdout).ok().map(|s| s.trim() == "active")
        })
        .unwrap_or(false);

    if service_exists && service_active {
        println!("✓ Halvor agent service is already configured and running");
        println!();
    } else {
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Setting up halvor agent service on primary node");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!();
        
        let web_port = if std::env::var("HALVOR_WEB_DIR").is_ok() {
            Some(13000)
        } else {
            None
        };
        
        if let Err(e) = agent_service::setup_agent_service(&exec, web_port) {
            eprintln!("⚠️  Warning: Failed to setup halvor agent service: {}", e);
            eprintln!("   You can set it up manually later with: halvor agent start --port 13500 --daemon");
        } else {
            println!("✓ Halvor agent service is running on {}", hostname);
            println!("  Agent API: port 13500 (over Tailscale)");
            if web_port.is_some() {
                println!("  Web UI: port 13000 (over Tailscale)");
            }
        }
        println!();
    }

    Ok(())
}
