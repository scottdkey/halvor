//! K3s kubeconfig management

use crate::config::EnvConfig;
use crate::services::tailscale;
use crate::utils::exec::{CommandExecutor, Executor, local};
use anyhow::{Context, Result};
use yaml_rust::YamlLoader;

/// Fetch kubeconfig content from environment variable (1Password) or primary node
/// Returns the kubeconfig content as a string, with localhost/127.0.0.1 replaced with Tailscale address
pub fn fetch_kubeconfig_content(primary_hostname: &str, config: &EnvConfig) -> Result<String> {
    println!("  Fetching kubeconfig...");

    // Helper function to process kubeconfig content - replaces localhost with Tailscale address
    let process_kubeconfig = |mut kubeconfig: String, hostname: &str| -> Result<String> {
        // Get Tailscale IP/hostname for the node to replace 127.0.0.1
        let exec = Executor::new(hostname, config)
            .with_context(|| format!("Failed to create executor for node: {}", hostname))?;
        let tailscale_ip =
            tailscale::get_tailscale_ip_with_fallback(&exec, hostname, config)?;
        let tailscale_hostname = tailscale::get_tailscale_hostname_remote(&exec)
            .ok()
            .flatten();

        // Replace localhost/127.0.0.1 with Tailscale address
        let server_address = tailscale_hostname.as_ref().unwrap_or(&tailscale_ip);
        kubeconfig = kubeconfig.replace("127.0.0.1", server_address);
        kubeconfig = kubeconfig.replace("localhost", server_address);

        Ok(kubeconfig)
    };

    // First, try to read from local K3s installation if running on this machine
    // But only use it if the primary_hostname matches the local machine
    // Otherwise, we need to fetch from the primary node
    if std::path::Path::new("/etc/rancher/k3s/k3s.yaml").exists() {
        // Check if primary_hostname matches local machine
        let is_local_primary = if let Ok(current_hostname) = crate::config::service::get_current_hostname() {
            let normalized_current = crate::config::service::normalize_hostname(&current_hostname);
            let normalized_primary = crate::config::service::normalize_hostname(primary_hostname);
            primary_hostname.eq_ignore_ascii_case(&current_hostname)
                || primary_hostname.eq_ignore_ascii_case(&normalized_current)
                || normalized_primary.eq_ignore_ascii_case(&normalized_current)
                || normalized_primary.eq_ignore_ascii_case(&current_hostname)
        } else {
            false
        };

        if is_local_primary {
            println!("  ✓ Found local K3s installation (primary node)");
            match local::execute_shell("sudo cat /etc/rancher/k3s/k3s.yaml") {
                Ok(output) if output.status.success() => {
                    let mut kubeconfig = String::from_utf8_lossy(&output.stdout).to_string();
                    println!("  ✓ Using kubeconfig from local K3s (/etc/rancher/k3s/k3s.yaml)");

                    // For local primary, we can use localhost or Tailscale address
                    // Check if we should use Tailscale address for remote access
                    let tailscale_status = local::execute_shell("tailscale status --json");
                    if let Ok(status_output) = tailscale_status {
                        if status_output.status.success() {
                            let status_str = String::from_utf8_lossy(&status_output.stdout);
                            if let Ok(status_json) = serde_json::from_str::<serde_json::Value>(&status_str) {
                                // Get Tailscale hostname from Self field
                                if let Some(ts_hostname) = status_json.get("Self")
                                    .and_then(|s| s.get("DNSName"))
                                    .and_then(|d| d.as_str())
                                    .map(|s| s.trim_end_matches('.').to_string()) {
                                    println!("  Using Tailscale hostname: {}", ts_hostname);
                                    kubeconfig = kubeconfig.replace("127.0.0.1", &ts_hostname);
                                    kubeconfig = kubeconfig.replace("localhost", &ts_hostname);
                                } else if let Some(ts_ip) = status_json.get("Self")
                                    .and_then(|s| s.get("TailscaleIPs"))
                                    .and_then(|ips| ips.as_array())
                                    .and_then(|arr| arr.get(0))
                                    .and_then(|ip| ip.as_str()) {
                                    println!("  Using Tailscale IP: {}", ts_ip);
                                    kubeconfig = kubeconfig.replace("127.0.0.1", ts_ip);
                                    kubeconfig = kubeconfig.replace("localhost", ts_ip);
                                }
                            }
                        }
                    }

                    return Ok(kubeconfig);
                }
                _ => {
                    println!("  ⚠ Local K3s found but couldn't read kubeconfig (need sudo access)");
                }
            }
        } else {
            println!("  ⚠ Local K3s found but primary is different node ({}), will fetch from primary", primary_hostname);
        }
    }

    // Next, try to get from KUBE_CONFIG environment variable (from 1Password)
    if let Ok(kubeconfig_content) = std::env::var("KUBE_CONFIG") {
        println!("  ✓ Using kubeconfig from KUBE_CONFIG environment variable");
        return process_kubeconfig(kubeconfig_content, primary_hostname);
    }

    // If primary is remote, try to fetch kubeconfig from the primary node
    let primary_exec = Executor::new(primary_hostname, config).ok();
    if let Some(exec) = primary_exec {
        if !exec.is_local() {
            println!("  Fetching kubeconfig from primary node ({})...", primary_hostname);
            match exec.execute_shell("sudo cat /etc/rancher/k3s/k3s.yaml") {
                Ok(output) if output.status.success() => {
                    let kubeconfig = String::from_utf8_lossy(&output.stdout).to_string();
                    println!("  ✓ Fetched kubeconfig from primary node");
                    
                    // Process the kubeconfig to replace localhost with primary's Tailscale address
                    return process_kubeconfig(kubeconfig, primary_hostname);
                }
                _ => {
                    println!("  ⚠ Could not fetch kubeconfig from primary node (need sudo access or K3s not installed)");
                }
            }
        }
    }

    // If not found locally or in environment, provide helpful error message
    anyhow::bail!(
        "Kubeconfig not found. Tried:\n\
         1. Local K3s installation (/etc/rancher/k3s/k3s.yaml)\n\
         2. KUBE_CONFIG environment variable (from 1Password)\n\
         3. Fetching from primary node ({})\n\
         \n\
         Please either:\n\
         - Ensure K3s is installed on the primary node, or\n\
         - Add the kubeconfig to your 1Password vault as KUBE_CONFIG=\"<content>\"",
        primary_hostname
    )
}

/// Process kubeconfig from environment variable to ensure it points to the primary server
/// This is a public function for use when KUBE_CONFIG is available
/// It parses the YAML and replaces the server URL with the primary server's address
pub fn process_kubeconfig_for_primary(
    kubeconfig: &str,
    primary_hostname: &str,
    config: &EnvConfig,
) -> Result<String> {
    process_kubeconfig_for_primary_internal(kubeconfig, primary_hostname, config, false)
}

fn process_kubeconfig_for_primary_internal(
    kubeconfig: &str,
    primary_hostname: &str,
    config: &EnvConfig,
    is_fallback: bool, // Prevent infinite recursion
) -> Result<String> {
    // Clean up the kubeconfig content from 1Password FIRST
    // We need to clean it before we can parse it to extract the server
    // 1Password may store multi-line values in various formats:
    // - Escaped strings (e.g., "apiVersion: v1\nkind: Config\n...")
    // - Quoted strings with escaped newlines
    // - With trailing '=' characters
    // - As base64
    let mut cleaned_content = kubeconfig.to_string();
    
    // Remove surrounding quotes if present (1Password might quote the entire value)
    cleaned_content = cleaned_content.trim().to_string();
    if (cleaned_content.starts_with('"') && cleaned_content.ends_with('"'))
        || (cleaned_content.starts_with('\'') && cleaned_content.ends_with('\'')) {
        cleaned_content = cleaned_content[1..cleaned_content.len()-1].to_string();
    }
    
    // First, try to unescape \n sequences (1Password might escape newlines)
    if cleaned_content.contains("\\n") && !cleaned_content.contains('\n') {
        // Looks like it's escaped - unescape common escape sequences
        cleaned_content = cleaned_content
            .replace("\\n", "\n")
            .replace("\\t", "\t")
            .replace("\\r", "\r")
            .replace("\\\"", "\"")
            .replace("\\'", "'")
            .replace("\\\\", "\\");
    }
    
    // Remove trailing '=' characters that 1Password might add to multi-line values
    cleaned_content = cleaned_content
        .lines()
        .map(|line| line.trim_end_matches('='))
        .collect::<Vec<_>>()
        .join("\n");
    
    // Try base64 decoding if it looks like base64 (all printable base64 chars, ends with = or ==)
    // But only if it doesn't already look like valid YAML
    if !cleaned_content.trim_start().starts_with("apiVersion") 
        && !cleaned_content.trim_start().starts_with("kind:")
        && cleaned_content.len() > 100 
        && cleaned_content.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=' || c == '\n' || c == '\r' || c == ' ')
        && cleaned_content.trim().ends_with('=') {
        // Might be base64 encoded - try decoding
        use base64::{Engine as _, engine::general_purpose};
        if let Ok(decoded) = general_purpose::STANDARD.decode(cleaned_content.trim()) {
            if let Ok(decoded_str) = String::from_utf8(decoded) {
                println!("  Detected base64-encoded kubeconfig, decoded");
                cleaned_content = decoded_str;
            }
        }
    }

    // Validate that the content looks like YAML before parsing
    if !cleaned_content.trim_start().starts_with("apiVersion") 
        && !cleaned_content.trim_start().starts_with("kind:") {
        // Doesn't look like YAML - might need more processing
        println!("  ⚠ Kubeconfig doesn't start with 'apiVersion' or 'kind:', may need additional processing");
        
        // Try to find YAML content if it's embedded in something else
        if let Some(api_start) = cleaned_content.find("apiVersion") {
            println!("  Found 'apiVersion' at position {}, extracting from there", api_start);
            cleaned_content = cleaned_content[api_start..].to_string();
        }
    }
    
    // Parse the kubeconfig YAML to extract current server
    // Provide better error context if parsing fails
    let docs = match YamlLoader::load_from_str(&cleaned_content) {
        Ok(docs) => docs,
        Err(e) => {
            // Try to extract line number from error if possible
            let error_msg = format!("{}", e);
            let preview = if cleaned_content.len() > 500 {
                format!("{}...", &cleaned_content[..500])
            } else {
                cleaned_content.clone()
            };
            let line_count = cleaned_content.lines().count();
            let first_line = cleaned_content.lines().next().unwrap_or("").to_string();
            let first_3_lines: Vec<&str> = cleaned_content.lines().take(3).collect();
            
            // Show character at position 70 (where the error occurred)
            let char_at_70 = if cleaned_content.len() > 70 {
                format!("Character at position 70: '{}' (0x{:02x})", 
                    cleaned_content.chars().nth(70).unwrap_or('?'),
                    cleaned_content.as_bytes().get(70).copied().unwrap_or(0))
            } else {
                "Content is shorter than 70 characters".to_string()
            };
            
            println!("  [DEBUG] Kubeconfig parsing failed. Details:");
            println!("    Error: {}", error_msg);
            println!("    Length: {} chars, Lines: {}", cleaned_content.len(), line_count);
            let first_line_display = if first_line.len() > 100 { format!("{}...", &first_line[..100]) } else { first_line.clone() };
            println!("    First line: {}", first_line_display);
            println!("    First 3 lines:");
            for (i, line) in first_3_lines.iter().enumerate() {
                let line_display = if line.len() > 100 { format!("{}...", &line[..100]) } else { line.to_string() };
                println!("      {}: {}", i+1, line_display);
            }
            println!("    {}", char_at_70);
            
            // If parsing fails and we haven't already tried fetching, try fetching directly from frigg as fallback
            if !is_fallback {
                println!("  Attempting to fetch kubeconfig directly from {} as fallback...", primary_hostname);
                match fetch_kubeconfig_content(primary_hostname, config) {
                    Ok(fetched_kubeconfig) => {
                        println!("  ✓ Successfully fetched kubeconfig from {}", primary_hostname);
                        // Recursively process the fetched kubeconfig (but mark as fallback to prevent infinite recursion)
                        return process_kubeconfig_for_primary_internal(&fetched_kubeconfig, primary_hostname, config, true);
                    }
                    Err(fetch_err) => {
                        anyhow::bail!(
                            "Failed to parse kubeconfig YAML from KUBE_CONFIG environment variable, and also failed to fetch from {}.\n\
                             \n\
                             Parse error: {}\n\
                             \n\
                             Content info:\n\
                             - Total length: {} characters\n\
                             - Number of lines: {}\n\
                             - First line: {}\n\
                             - First 500 chars: {}\n\
                             - {}\n\
                             \n\
                             Fetch error: {}\n\
                             \n\
                             The kubeconfig from 1Password may be incorrectly formatted.\n\
                             Common issues:\n\
                             - Stored as a single quoted/escaped string instead of multi-line\n\
                             - Has extra quotes or escape characters that break YAML syntax\n\
                             - Missing actual newlines (all on one line with \\n sequences)\n\
                             - Contains invalid YAML characters\n\
                             \n\
                             Solution:\n\
                             Ensure KUBE_CONFIG in 1Password is stored as a multi-line text field.\n\
                             The value should start with 'apiVersion: v1' and have actual line breaks,\n\
                             not escaped \\n sequences.",
                            primary_hostname,
                            error_msg, cleaned_content.len(), line_count, 
                            if first_line.len() > 100 { format!("{}...", &first_line[..100]) } else { first_line },
                            preview,
                            char_at_70,
                            fetch_err
                        );
                    }
                }
            } else {
                // Already tried fetching, just bail with the parse error
                anyhow::bail!(
                    "Failed to parse kubeconfig YAML.\n\
                     \n\
                     Error: {}\n\
                     \n\
                     Content info:\n\
                     - Total length: {} characters\n\
                     - Number of lines: {}\n\
                     - First line: {}\n\
                     - First 500 chars: {}\n\
                     - {}\n\
                     \n\
                     The kubeconfig may be incorrectly formatted.",
                    error_msg, cleaned_content.len(), line_count, 
                    if first_line.len() > 100 { format!("{}...", &first_line[..100]) } else { first_line },
                    preview,
                    char_at_70
                );
            }
        }
    };

    let doc = docs
        .first()
        .ok_or_else(|| anyhow::anyhow!("Empty kubeconfig"))?;

    // Extract the server URL from the parsed YAML - no SSH needed!
    // This avoids needing to SSH to the primary node
    let existing_server = doc["clusters"][0]["cluster"]["server"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No server found in kubeconfig clusters"))?;
    
    // Parse the server URL to get just the hostname/IP (remove https:// and :6443)
    let existing_server_host = existing_server.trim();
    
    // Use the server from kubeconfig as the primary server address
    let primary_server_address = existing_server_host
        .strip_prefix("https://")
        .or_else(|| existing_server_host.strip_prefix("http://"))
        .and_then(|s| s.split(':').next())
        .unwrap_or(existing_server_host);
    
    let primary_server_url = if existing_server_host.starts_with("https://") {
        existing_server_host.to_string()
    } else {
        format!("https://{}:6443", existing_server_host)
    };

    println!("  Detected server from kubeconfig: {}", existing_server_host);
    println!("  Primary server URL: {}", primary_server_url);

    // Find ALL server URLs in the kubeconfig (there may be multiple clusters)
    let mut servers_to_replace = Vec::new();
    if let Some(clusters) = doc["clusters"].as_vec() {
        for cluster in clusters {
            if let Some(server) = cluster["cluster"]["server"].as_str() {
                if server != &primary_server_url {
                    servers_to_replace.push(server.to_string());
                }
            }
        }
    }
    
    // If no clusters found, try to extract from first cluster
    if servers_to_replace.is_empty() {
        if let Some(server) = doc["clusters"][0]["cluster"]["server"].as_str() {
            if server != &primary_server_url {
                servers_to_replace.push(server.to_string());
            }
        }
    }

    println!("  Found {} server URL(s) to replace", servers_to_replace.len());
    for server in &servers_to_replace {
        println!("    - {}", server);
    }
    println!("  Replacing with primary server: {}", primary_server_url);

    // Use comprehensive string replacement - more reliable than trying to mutate YAML
    // yaml_rust doesn't support IndexMut, so we'll do string replacement on the cleaned content
    let mut processed = cleaned_content;
    
    // Replace ALL server URLs found in the kubeconfig using multiple patterns
    for current_server in &servers_to_replace {
        // Pattern 1: "server: https://hostname:6443" (with space after colon)
        let pattern1 = format!("server: {}", current_server);
        if processed.contains(&pattern1) {
            processed = processed.replace(&pattern1, &format!("server: {}", primary_server_url));
            println!("    ✓ Replaced: {}", pattern1);
        }
        
        // Pattern 2: "server:https://hostname:6443" (no space after colon)
        let pattern2 = format!("server:{}", current_server);
        if processed.contains(&pattern2) {
            processed = processed.replace(&pattern2, &format!("server: {}", primary_server_url));
            println!("    ✓ Replaced: {}", pattern2);
        }
        
        // Pattern 3: Direct URL replacement (fallback - replace anywhere)
        if processed.contains(current_server) {
            processed = processed.replace(current_server, &primary_server_url);
            println!("    ✓ Replaced direct URL: {}", current_server);
        }
    }
    
    // Also replace any hostnames/IPs that might be the joining node
    // Extract hostname/IP from server URLs to replace
    for current_server in &servers_to_replace {
        // Extract hostname from URL (e.g., "https://baulder.bombay-pinecone.ts.net:6443" -> "baulder.bombay-pinecone.ts.net")
        if let Some(host_part) = current_server.strip_prefix("https://").or_else(|| current_server.strip_prefix("http://")) {
            if let Some(hostname) = host_part.split(':').next() {
                // Replace hostname in any form (but not if it's the primary server address)
                if processed.contains(hostname) && hostname != primary_server_address {
                    // Only replace if it's part of a URL or server field
                    processed = processed.replace(&format!("https://{}:6443", hostname), &primary_server_url);
                    processed = processed.replace(&format!("http://{}:6443", hostname), &primary_server_url);
                    processed = processed.replace(&format!("{}:6443", hostname), &format!("{}:6443", primary_server_address));
                    println!("    ✓ Replaced hostname: {}", hostname);
                }
            }
        }
    }

    // Also do aggressive string replacement as a fallback for any remaining occurrences
    // This catches cases where the YAML structure might have been different
    for current_server in &servers_to_replace {
        if processed.contains(current_server) {
            processed = processed.replace(current_server, &primary_server_url);
            println!("    ✓ Fallback string replacement: {}", current_server);
        }
    }
    
    // Replace any hostnames/IPs that might be the joining node
    // Extract hostname/IP from server URLs to replace
    for current_server in &servers_to_replace {
        // Extract hostname from URL (e.g., "https://baulder.bombay-pinecone.ts.net:6443" -> "baulder.bombay-pinecone.ts.net")
        if let Some(host_part) = current_server.strip_prefix("https://").or_else(|| current_server.strip_prefix("http://")) {
            if let Some(hostname) = host_part.split(':').next() {
                // Replace hostname in any form
                if processed.contains(hostname) && hostname != primary_server_address {
                    processed = processed.replace(hostname, primary_server_address);
                    println!("    ✓ Replaced hostname: {}", hostname);
                }
            }
        }
    }
    
    // Also replace localhost/127.0.0.1 as additional safety
    processed = processed.replace("127.0.0.1:6443", &primary_server_url);
    processed = processed.replace("localhost:6443", &primary_server_url);
    processed = processed.replace("127.0.0.1", primary_server_address);
    processed = processed.replace("localhost", primary_server_address);

    // Verify the replacement worked by parsing the processed kubeconfig
    let verify_docs = YamlLoader::load_from_str(&processed).ok();
    if let Some(verify_docs) = verify_docs {
        if let Some(verify_doc) = verify_docs.first() {
            if let Some(clusters) = verify_doc["clusters"].as_vec() {
                for (idx, cluster) in clusters.iter().enumerate() {
                    if let Some(server) = cluster["cluster"]["server"].as_str() {
                        if server == &primary_server_url {
                            println!("  ✓ Verified cluster {} points to primary server", idx);
                        } else {
                            println!("  ⚠ Warning: Cluster {} still has server: {} (expected: {})", idx, server, primary_server_url);
                            // Try one more aggressive replacement
                            processed = processed.replace(server, &primary_server_url);
                        }
                    }
                }
            }
        }
    }

    Ok(processed)
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
#[allow(dead_code)]
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

/// Extract server URL and token from kubeconfig
/// Returns (server_hostname, token)
pub fn extract_server_and_token_from_kubeconfig(kubeconfig_content: &str) -> Result<(String, String)> {
    // Clean up the kubeconfig content - remove trailing '=' characters that 1Password might add
    // These appear on multi-line values and break YAML parsing
    let cleaned_content = kubeconfig_content
        .lines()
        .map(|line| line.trim_end_matches('='))
        .collect::<Vec<_>>()
        .join("\n");

    // Parse the kubeconfig YAML
    let docs = YamlLoader::load_from_str(&cleaned_content)
        .context("Failed to parse kubeconfig YAML")?;

    let doc = docs
        .first()
        .ok_or_else(|| anyhow::anyhow!("Empty kubeconfig"))?;

    // Extract server URL from clusters[0].cluster.server
    let server = doc["clusters"][0]["cluster"]["server"]
        .as_str()
        .ok_or_else(|| {
            // Provide helpful debugging info
            let clusters = &doc["clusters"];
            anyhow::anyhow!(
                "No server found in kubeconfig clusters. Clusters structure: {:?}",
                clusters
            )
        })?;

    // Extract token from users[0].user.token
    let token = doc["users"][0]["user"]["token"]
        .as_str()
        .or_else(|| {
            // Also try client-certificate-data if token is not present
            doc["users"][0]["user"]["client-certificate-data"].as_str()
        })
        .ok_or_else(|| anyhow::anyhow!("No token or client-certificate-data found in kubeconfig users"))?
        .to_string();

    // Parse server URL to extract hostname (remove https:// and port)
    let server_host = server
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split(':')
        .next()
        .ok_or_else(|| anyhow::anyhow!("Invalid server URL in kubeconfig"))?
        .to_string();

    Ok((server_host, token))
}
