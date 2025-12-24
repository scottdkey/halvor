//! Download PIA OpenVPN configs

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

const CONFIG_DIR: &str = "/config";
const PIA_CONFIG_URL: &str = "https://www.privateinternetaccess.com/openvpn/openvpn.zip";

/// Download PIA OpenVPN configs if UPDATE_CONFIGS is set
pub fn download_pia_configs() -> Result<()> {
    let region = std::env::var("REGION").ok();
    let region_normalized = region.as_ref().map(|r| r.to_lowercase().replace('-', "_"));

    if let Some(ref region) = region_normalized {
        println!(
            "UPDATE_CONFIGS is set - downloading PIA config for region: {}",
            region
        );
    } else {
        println!(
            "UPDATE_CONFIGS is set - downloading all PIA OpenVPN configs (no REGION specified)"
        );
    }

    // Ensure /config directory exists and is writable
    let config_dir = Path::new(CONFIG_DIR);
    if !config_dir.exists() {
        fs::create_dir_all(config_dir)?;
    }

    // Set permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(config_dir)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(config_dir, perms)?;
    }

    // Check if config directory is writable
    let metadata = fs::metadata(config_dir)?;
    if metadata.permissions().readonly() {
        eprintln!("⚠ Warning: /config is not writable, cannot download configs");
        eprintln!("  Make sure the volume mount is not read-only (:ro)");
        return Ok(());
    }

    // Create temp directory for download
    let temp_dir = tempfile::tempdir()?;
    let zip_path = temp_dir.path().join("openvpn.zip");

    println!("Downloading PIA OpenVPN configs from: {}", PIA_CONFIG_URL);

    // Download the zip file
    let response =
        reqwest::blocking::get(PIA_CONFIG_URL).context("Failed to download PIA configs")?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to download configs: HTTP {}", response.status());
    }

    let mut zip_file = fs::File::create(&zip_path)?;
    let mut content = std::io::Cursor::new(response.bytes()?);
    std::io::copy(&mut content, &mut zip_file)?;
    drop(zip_file);

    println!("✓ Download successful");

    // Extract zip file
    let zip_file = fs::File::open(&zip_path)?;
    let mut archive = zip::ZipArchive::new(zip_file)?;

    if let Some(ref region) = region_normalized {
        // Extract only the specified region's config
        let config_file = format!("{}.ovpn", region);

        // First, collect all available config file names to check if our target exists
        let available_configs: Vec<String> = {
            let mut configs = Vec::new();
            for i in 0..archive.len() {
                if let Ok(file) = archive.by_index(i) {
                    let name = file.name();
                    if name.ends_with(".ovpn") {
                        configs.push(name.to_string());
                    }
                }
            }
            configs
        };

        // Check if the requested config exists
        if available_configs.iter().any(|name| name == &config_file) {
            // Extract the specific config file
            let mut file = archive.by_name(&config_file)?;
            let mut config_content = Vec::new();
            std::io::copy(&mut file, &mut config_content)?;
            drop(file); // Explicitly drop to release the borrow

            let dest_path = config_dir.join(&config_file);
            fs::write(&dest_path, config_content)?;
            println!("✓ Extracted: {}", config_file);
            println!("✓ Config copied to /config/{}", config_file);
        } else {
            // File not found - list available configs and fall back to extracting all
            eprintln!("⚠ Config '{}' not found in archive", config_file);
            eprintln!("  Available configs:");
            for name in &available_configs {
                println!("    {}", name);
            }
            eprintln!("  Falling back to extracting all configs...");
            extract_all_configs(&mut archive, config_dir)?;
        }
    } else {
        // Extract all configs when no region specified
        extract_all_configs(&mut archive, config_dir)?;
    }

    Ok(())
}

fn extract_all_configs(archive: &mut zip::ZipArchive<fs::File>, config_dir: &Path) -> Result<()> {
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name();

        if name.ends_with(".ovpn") {
            let dest_path = config_dir.join(
                Path::new(name)
                    .file_name()
                    .ok_or_else(|| anyhow::anyhow!("Invalid filename: {}", name))?,
            );

            let mut config_content = Vec::new();
            std::io::copy(&mut file, &mut config_content)?;
            fs::write(&dest_path, config_content)?;
        }
    }

    println!("✓ Extraction successful");
    println!("✓ All configs copied to /config");
    Ok(())
}
