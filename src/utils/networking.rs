use crate::utils::exec::local;
use anyhow::Result;

/// Get all local IP addresses
pub fn get_local_ips() -> Result<Vec<String>> {
    let mut ips = Vec::new();

    // Try to get IPs using platform-specific commands
    #[cfg(unix)]
    {
        // Use `hostname -I` on Linux or `ifconfig` on macOS
        if let Ok(output) = local::execute("hostname", &["-I"]) {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for ip in stdout.split_whitespace() {
                ips.push(ip.to_string());
            }
        }

        // Also try `ip addr` on Linux
        if let Ok(output) = local::execute("ip", &["addr", "show"]) {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("inet ") && !line.contains("127.0.0.1") && !line.contains("::1") {
                    if let Some(ip_part) = line.split_whitespace().nth(1) {
                        if let Some(ip) = ip_part.split('/').next() {
                            ips.push(ip.to_string());
                        }
                    }
                }
            }
        }
    }

    #[cfg(windows)]
    {
        // Use `ipconfig` on Windows
        if let Ok(output) = local::execute("ipconfig", &[]) {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("IPv4 Address") || line.contains("IPv4 地址") {
                    if let Some(ip_part) = line.split(':').nth(1) {
                        let ip = ip_part.trim();
                        if !ip.is_empty() {
                            ips.push(ip.to_string());
                        }
                    }
                }
            }
        }
    }

    Ok(ips)
}
