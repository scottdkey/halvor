//! Configuration file management

use anyhow::{Context, Result};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

const CONFIG_DIR: &str = "/config";

/// Fix all existing config files for OpenVPN 2.6 compatibility
pub fn fix_config_files() -> Result<()> {
    let config_dir = Path::new(CONFIG_DIR);
    if !config_dir.exists() {
        return Ok(());
    }

    let ovpn_files: Vec<PathBuf> = fs::read_dir(config_dir)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension() == Some(std::ffi::OsStr::new("ovpn")) {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    if ovpn_files.is_empty() {
        return Ok(());
    }

    println!("Fixing configs for OpenVPN 2.6 compatibility...");

    for ovpn_path in &ovpn_files {
        let filename = ovpn_path.file_name().unwrap().to_string_lossy();
        println!("  Processing: {}", filename);

        let content = fs::read_to_string(ovpn_path)
            .with_context(|| format!("Failed to read {}", ovpn_path.display()))?;

        // Fix line endings (CRLF -> LF)
        let content = content.replace("\r\n", "\n").replace('\r', "\n");

        // Remove <crl-verify>...</crl-verify> block (multi-line)
        let crl_block_re = Regex::new(r"(?s)<crl-verify>.*?</crl-verify>").unwrap();
        let content = crl_block_re.replace_all(&content, "").to_string();

        // Remove standalone crl-verify directive
        let crl_directive_re = Regex::new(r"(?m)^crl-verify.*$").unwrap();
        let content = crl_directive_re.replace_all(&content, "").to_string();

        // Remove IPv6 directives
        let ipv6_re = Regex::new(r"(?m)^(ifconfig-ipv6|route-ipv6).*$").unwrap();
        let content = ipv6_re.replace_all(&content, "").to_string();

        fs::write(ovpn_path, content)
            .with_context(|| format!("Failed to write {}", ovpn_path.display()))?;
    }

    println!("✓ Configs fixed");
    Ok(())
}

/// Create auth.txt from environment variables
pub fn create_auth_file() -> Result<()> {
    let username = std::env::var("PIA_USERNAME").ok();
    let password = std::env::var("PIA_PASSWORD").ok();

    if username.is_none() || password.is_none() {
        if !Path::new("/config/auth.txt").exists() {
            eprintln!("⚠ Warning: /config/auth.txt not found");
            eprintln!("OpenVPN may fail without authentication credentials");
            eprintln!();
            eprintln!("Options:");
            eprintln!("  1. Set PIA_USERNAME and PIA_PASSWORD environment variables");
            eprintln!("  2. Create /config/auth.txt manually with format:");
            eprintln!("     Line 1: PIA username");
            eprintln!("     Line 2: PIA password");
        }
        return Ok(());
    }

    let username = username.unwrap();
    let password = password.unwrap();

    println!("PIA_USERNAME and PIA_PASSWORD provided - creating/updating auth.txt...");

    let config_dir = Path::new(CONFIG_DIR);
    if !config_dir.exists() {
        fs::create_dir_all(config_dir)?;
    }

    // Check if config directory is writable
    let metadata = fs::metadata(config_dir)?;
    if metadata.permissions().readonly() {
        eprintln!("⚠ Warning: Cannot write to /config (volume may be read-only)");
        eprintln!("  Please ensure volume mount is writable or provide auth.txt manually");
        eprintln!(
            "  Remove :ro from volume mount if using UPDATE_CONFIGS or PIA_USERNAME/PIA_PASSWORD"
        );
        return Ok(());
    }

    let auth_path = config_dir.join("auth.txt");
    let mut auth_file = fs::File::create(&auth_path)?;
    use std::io::Write;
    writeln!(auth_file, "{}", username)?;
    writeln!(auth_file, "{}", password)?;

    // Set permissions to 600 (read/write for owner only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&auth_path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&auth_path, perms)?;
    }

    println!("✓ auth.txt created/updated from environment variables");
    Ok(())
}

/// Find OpenVPN config file based on REGION or default
pub fn find_ovpn_config() -> Result<PathBuf> {
    let config_dir = Path::new(CONFIG_DIR);

    println!("Checking for OpenVPN config files in /config...");
    println!("Contents of /config:");
    if config_dir.exists() {
        match fs::read_dir(config_dir) {
            Ok(entries) => {
                for entry in entries {
                    if let Ok(entry) = entry {
                        println!("  {}", entry.file_name().to_string_lossy());
                    }
                }
            }
            Err(e) => {
                eprintln!("Cannot list /config directory: {}", e);
            }
        }
    } else {
        eprintln!("Cannot list /config directory");
    }

    let region = std::env::var("REGION").ok();

    if let Some(ref region) = region {
        println!("REGION specified: {}", region);
        let region_normalized = region.to_lowercase().replace('-', "_");
        println!("Normalized region: {}", region_normalized);

        // Try exact match first
        let exact_path = config_dir.join(format!("{}.ovpn", region_normalized));
        if exact_path.exists() {
            println!("✓ Found exact match: {}", exact_path.display());
            return Ok(exact_path);
        }

        // Try partial match
        if let Ok(entries) = fs::read_dir(config_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.extension() == Some(std::ffi::OsStr::new("ovpn")) {
                        let filename = path.file_stem().unwrap().to_string_lossy().to_lowercase();
                        if filename.contains(&region_normalized) {
                            println!("✓ Found partial match: {}", path.display());
                            return Ok(path);
                        }
                    }
                }
            }
        }

        // Try matching just the last part
        let region_parts: Vec<&str> = region_normalized.split(&['_', '-'][..]).collect();
        if let Some(last_part) = region_parts.last() {
            if let Ok(entries) = fs::read_dir(config_dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.extension() == Some(std::ffi::OsStr::new("ovpn")) {
                            let filename =
                                path.file_stem().unwrap().to_string_lossy().to_lowercase();
                            if filename.contains(last_part) {
                                println!(
                                    "✓ Found match by region part '{}': {}",
                                    last_part,
                                    path.display()
                                );
                                return Ok(path);
                            }
                        }
                    }
                }
            }
        }

        eprintln!("⚠ No config found matching region: {}", region);
        eprintln!("  Tried: {}.ovpn", region_normalized);
        eprintln!("  Tried: partial match for '{}'", region_normalized);
        if let Some(last_part) = region_parts.last() {
            if last_part != &region_normalized {
                eprintln!("  Tried: partial match for '{}'", last_part);
            }
        }
        eprintln!();
        eprintln!("Available configs (first 20):");
        if let Ok(entries) = fs::read_dir(config_dir) {
            let mut count = 0;
            for entry in entries {
                if count >= 20 {
                    break;
                }
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.extension() == Some(std::ffi::OsStr::new("ovpn")) {
                        println!("  - {}", path.file_name().unwrap().to_string_lossy());
                        count += 1;
                    }
                }
            }
        } else {
            println!("  (none found)");
        }
    }

    // Fallback: use first available .ovpn file (alphabetically sorted)
    if let Ok(entries) = fs::read_dir(config_dir) {
        let mut ovpn_files: Vec<PathBuf> = entries
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.extension() == Some(std::ffi::OsStr::new("ovpn")) {
                    Some(path)
                } else {
                    None
                }
            })
            .collect();

        if !ovpn_files.is_empty() {
            ovpn_files.sort();
            let first = ovpn_files[0].clone();
            if region.is_none() {
                println!(
                    "No REGION specified - using first available config: {}",
                    first.display()
                );
            }
            return Ok(first);
        }
    }

    // No config found
    eprintln!("⚠ No OpenVPN config file found in /config");
    eprintln!();
    eprintln!("Debugging information:");
    eprintln!("  /config exists: {}", config_dir.exists());
    if config_dir.exists() {
        if let Ok(metadata) = fs::metadata(config_dir) {
            eprintln!(
                "  /config readable: {}",
                metadata.permissions().readonly() == false
            );
            eprintln!(
                "  /config writable: {}",
                metadata.permissions().readonly() == false
            );
        }
    }
    eprintln!("  Files in /config:");
    if let Ok(entries) = fs::read_dir(config_dir) {
        let mut count = 0;
        for entry in entries {
            if count >= 10 {
                break;
            }
            if let Ok(entry) = entry {
                println!("    {}", entry.path().display());
                count += 1;
            }
        }
    } else {
        eprintln!("    Cannot search /config");
    }
    eprintln!();
    eprintln!("Please ensure:");
    eprintln!("  1. Directory $HOME/config/vpn exists on the host");
    eprintln!(
        "  2. Files are present: $HOME/config/vpn/<region>.ovpn and $HOME/config/vpn/auth.txt"
    );
    eprintln!("  3. Docker daemon has access to $HOME/config/vpn (check volume mount path)");
    eprintln!("  4. Or set UPDATE_CONFIGS=true to download configs automatically");
    eprintln!(
        "  5. Set REGION environment variable to select a specific region (e.g., REGION=us-california)"
    );

    anyhow::bail!("No OpenVPN config file found");
}

/// Configure Privoxy to listen on specified port
pub fn configure_privoxy(port: &str) -> Result<()> {
    println!("Configuring Privoxy to listen on port {}...", port);

    const PRIVOXY_CONFIG: &str = "/etc/privoxy/config";
    
    // Read existing config
    let config = if Path::new(PRIVOXY_CONFIG).exists() {
        fs::read_to_string(PRIVOXY_CONFIG)?
    } else {
        String::new()
    };

    // Remove existing listen-address lines
    let lines: Vec<&str> = config.lines().collect();
    let filtered: Vec<&str> = lines
        .iter()
        .filter(|line| !line.trim_start().starts_with("listen-address"))
        .copied()
        .collect();

    // Rebuild config with new listen-address
    let mut new_config = filtered.join("\n");
    if !new_config.ends_with('\n') {
        new_config.push('\n');
    }
    new_config.push_str(&format!("listen-address 0.0.0.0:{}\n", port));

    fs::write(PRIVOXY_CONFIG, new_config)?;
    Ok(())
}

