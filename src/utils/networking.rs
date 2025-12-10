use crate::utils::exec::local;
use anyhow::Result;

/// Get all local IP addresses
pub fn get_local_ips() -> Result<Vec<String>> {
    let mut ips = Vec::new();

    // Try to get IPs using platform-specific commands
    #[cfg(unix)]
    {
        // Try `ip addr` first (Linux) - most reliable
        if let Ok(output) = local::execute("ip", &["addr", "show"]) {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("inet ") && !line.contains("127.0.0.1") && !line.contains("::1") {
                    if let Some(ip_part) = line.split_whitespace().nth(1) {
                        if let Some(ip) = ip_part.split('/').next() {
                            let ip = ip.trim().to_string();
                            // Filter out Tailscale IPs (100.x.x.x) and loopback
                            if !ip.is_empty()
                                && !ip.starts_with("100.")
                                && !ip.starts_with("127.")
                                && !ips.contains(&ip)
                            {
                                ips.push(ip);
                            }
                        }
                    }
                }
            }
        }

        // Try `hostname -I` on Linux (gives all IPs)
        if let Ok(output) = local::execute("hostname", &["-I"]) {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for ip in stdout.split_whitespace() {
                let ip = ip.trim().to_string();
                // Filter out Tailscale IPs (100.x.x.x) and loopback
                if !ip.is_empty()
                    && !ip.starts_with("100.")
                    && !ip.starts_with("127.")
                    && !ip.starts_with("::1")
                    && !ips.contains(&ip)
                {
                    ips.push(ip);
                }
            }
        }

        // Try `ifconfig` on macOS/Linux (fallback)
        if let Ok(output) = local::execute("ifconfig", &[]) {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("inet ") && !line.contains("127.0.0.1") && !line.contains("::1") {
                    // Parse "inet 192.168.1.1 netmask..." format
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if let Some(ip) = parts.get(1) {
                        let ip = ip.trim().to_string();
                        // Filter out Tailscale IPs (100.x.x.x) and loopback
                        if !ip.is_empty()
                            && !ip.starts_with("100.")
                            && !ip.starts_with("127.")
                            && !ips.contains(&ip)
                        {
                            ips.push(ip);
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

/// Get Tailscale IP addresses (100.x.x.x range)
pub fn get_tailscale_ips() -> Result<Vec<String>> {
    let mut ips = Vec::new();

    // Try to get IPs using platform-specific commands
    #[cfg(unix)]
    {
        // Try `ip addr` first (Linux) - most reliable
        if let Ok(output) = local::execute("ip", &["addr", "show"]) {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("inet ") && line.contains("100.") {
                    if let Some(ip_part) = line.split_whitespace().nth(1) {
                        if let Some(ip) = ip_part.split('/').next() {
                            let ip = ip.trim().to_string();
                            if !ip.is_empty() && ip.starts_with("100.") && !ips.contains(&ip) {
                                ips.push(ip);
                            }
                        }
                    }
                }
            }
        }

        // Try `hostname -I` on Linux (gives all IPs)
        if let Ok(output) = local::execute("hostname", &["-I"]) {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for ip in stdout.split_whitespace() {
                let ip = ip.trim().to_string();
                if !ip.is_empty() && ip.starts_with("100.") && !ips.contains(&ip) {
                    ips.push(ip);
                }
            }
        }

        // Try `ifconfig` on macOS/Linux (fallback)
        if let Ok(output) = local::execute("ifconfig", &[]) {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("inet ") && line.contains("100.") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if let Some(ip) = parts.get(1) {
                        let ip = ip.trim().to_string();
                        if !ip.is_empty() && ip.starts_with("100.") && !ips.contains(&ip) {
                            ips.push(ip);
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
                if (line.contains("IPv4 Address") || line.contains("IPv4 地址"))
                    && line.contains("100.")
                {
                    if let Some(ip_part) = line.split(':').nth(1) {
                        let ip = ip_part.trim();
                        if !ip.is_empty() && ip.starts_with("100.") {
                            ips.push(ip.to_string());
                        }
                    }
                }
            }
        }
    }

    Ok(ips)
}
