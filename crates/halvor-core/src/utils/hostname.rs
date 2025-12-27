// Hostname utilities
// Extracted from config::service to avoid circular dependencies

use anyhow::Result;

/// Get the current hostname from the system
pub fn get_current_hostname() -> Result<String> {
    std::env::var("HOSTNAME")
        .or_else(|_| {
            std::process::Command::new("hostname")
                .output()
                .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
                .map_err(|e| anyhow::anyhow!("Failed to get hostname: {}", e))
        })
}

/// Normalize hostname by removing domain suffixes like .ts.net, .local, etc.
pub fn normalize_hostname(hostname: &str) -> String {
    hostname
        .trim()
        .split('.')
        .next()
        .unwrap_or(hostname)
        .to_lowercase()
}

/// Find hostname in config by checking various forms (normalized, with domain, etc.)
pub fn find_hostname_in_config(
    hostname: &str,
    config: &crate::config::EnvConfig,
) -> Option<String> {
    let normalized = normalize_hostname(hostname);
    
    // Try exact match first
    if config.hosts.contains_key(hostname) {
        return Some(hostname.to_string());
    }
    
    // Try normalized match
    if config.hosts.contains_key(&normalized) {
        return Some(normalized);
    }
    
    // Try finding by checking if any config key normalizes to the input
    for key in config.hosts.keys() {
        if normalize_hostname(key) == normalized {
            return Some(key.clone());
        }
    }
    
    None
}
