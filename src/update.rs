use crate::exec::local;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::env;
use std::io::Write;

const GITHUB_API_BASE: &str = "https://api.github.com";
const REPO_OWNER: &str = "scottdkey"; // TODO: Make this configurable
const REPO_NAME: &str = "homelab";

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    #[serde(skip)]
    _assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
struct Asset {
    #[serde(skip)]
    _name: String,
    #[serde(skip)]
    _browser_download_url: String,
}

pub fn check_for_updates(current_version: &str) -> Result<Option<String>> {
    // Skip update check in development mode
    if env::var("HAL_DEV_MODE").is_ok() || cfg!(debug_assertions) {
        return Ok(None);
    }

    let client = reqwest::blocking::Client::builder()
        .user_agent("hal-cli")
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .context("Failed to create HTTP client")?;

    let url = format!(
        "{}/repos/{}/{}/releases/latest",
        GITHUB_API_BASE, REPO_OWNER, REPO_NAME
    );

    let response = client
        .get(&url)
        .send()
        .context("Failed to fetch latest release")?;

    if !response.status().is_success() {
        // Silently fail - network issues shouldn't block the CLI
        return Ok(None);
    }

    let release: Release = response.json().context("Failed to parse release JSON")?;

    // Normalize versions by removing 'v' prefix for comparison
    let latest_version = release.tag_name.trim_start_matches('v');
    let current_version_normalized = current_version.trim_start_matches('v');

    // Compare versions (simple string comparison, assumes semver)
    if latest_version != current_version_normalized && latest_version > current_version_normalized {
        return Ok(Some(release.tag_name));
    }

    Ok(None)
}

pub fn prompt_for_update(new_version: &str, current_version: &str) -> Result<bool> {
    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Update Available!");
    println!("  Current version: {}", current_version);
    println!("  Latest version:  {}", new_version);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    print!("Would you like to download and install the update? [y/N]: ");
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    let response = input.trim().to_lowercase();
    Ok(response == "y" || response == "yes")
}

pub fn download_and_install_update(version: &str) -> Result<()> {
    println!("Downloading update...");

    // Detect platform
    let platform = if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        anyhow::bail!("Unsupported platform for auto-update");
    };

    // Map architecture to release format (x86_64 -> amd64, aarch64 -> arm64)
    let arch = if cfg!(target_arch = "x86_64") {
        "amd64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        anyhow::bail!("Unsupported architecture for auto-update");
    };

    // Release artifacts are named: hal-{version}-{platform}-{arch}.tar.gz or .zip
    let extension = if cfg!(target_os = "windows") {
        ".zip"
    } else {
        ".tar.gz"
    };

    let asset_name = format!(
        "hal-{}-{}-{}{}",
        version.trim_start_matches('v'),
        platform,
        arch,
        extension
    );
    let download_url = format!(
        "https://github.com/{}/{}/releases/download/{}/{}",
        REPO_OWNER, REPO_NAME, version, asset_name
    );

    println!("Downloading from: {}", download_url);

    // Get current executable path
    let current_exe = env::current_exe().context("Failed to get current executable path")?;
    let backup_path = current_exe.with_extension(format!("{}.bak", extension));

    // Download to temp file
    let client = reqwest::blocking::Client::builder()
        .user_agent("hal-cli")
        .build()
        .context("Failed to create HTTP client")?;

    let response = client
        .get(&download_url)
        .send()
        .context("Failed to download update")?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to download update: HTTP {}", response.status());
    }

    let temp_archive = std::env::temp_dir().join(format!("hal-update-{}{}", version, extension));
    let mut file = std::fs::File::create(&temp_archive).context("Failed to create temp file")?;
    std::io::copy(&mut response.bytes()?.as_ref(), &mut file)
        .context("Failed to write download")?;
    drop(file);

    println!("Extracting archive...");

    // Extract the archive
    let temp_dir = std::env::temp_dir().join(format!("hal-update-extract-{}", version));
    local::create_dir_all(&temp_dir)?;

    let extracted_binary = if cfg!(target_os = "windows") {
        // Extract ZIP file
        let archive = std::fs::File::open(&temp_archive).context("Failed to open archive")?;
        let mut zip = zip::ZipArchive::new(archive).context("Failed to read ZIP archive")?;

        // Find the hal.exe file in the archive
        let mut found = false;
        let mut binary_path = None;
        for i in 0..zip.len() {
            let mut file = zip.by_index(i).context("Failed to read ZIP entry")?;
            let name = file.name().to_string();

            if name.ends_with("hal.exe") || name == "hal.exe" {
                let out_path = temp_dir.join("hal.exe");
                let mut out_file =
                    std::fs::File::create(&out_path).context("Failed to create output file")?;
                std::io::copy(&mut file, &mut out_file).context("Failed to extract file")?;
                binary_path = Some(out_path);
                found = true;
                break;
            }
        }

        if !found {
            anyhow::bail!("hal.exe not found in ZIP archive");
        }
        binary_path.unwrap()
    } else {
        // Extract tar.gz file
        use flate2::read::GzDecoder;
        use tar::Archive;

        let archive_file = std::fs::File::open(&temp_archive).context("Failed to open archive")?;
        let decoder = GzDecoder::new(archive_file);
        let mut archive = Archive::new(decoder);

        archive
            .unpack(&temp_dir)
            .context("Failed to extract tar.gz archive")?;

        // Find the hal binary in the extracted files
        let binary_path = temp_dir.join("hal");
        if !local::path_exists(&binary_path) {
            // Try looking in subdirectories
            let mut found = false;
            let entries = local::list_directory(&temp_dir)?;
            for entry_name in entries {
                let path = temp_dir.join(&entry_name);
                if local::is_directory(&path) {
                    let candidate = path.join("hal");
                    if local::path_exists(&candidate) {
                        local::copy_file(&candidate, &binary_path)?;
                        found = true;
                        break;
                    }
                } else if entry_name == "hal" {
                    local::copy_file(&path, &binary_path)?;
                    found = true;
                    break;
                }
            }
            if !found {
                anyhow::bail!("hal binary not found in extracted archive");
            }
        }
        binary_path
    };

    // Make executable (Unix)
    #[cfg(unix)]
    {
        local::set_permissions(&extracted_binary, 0o755)?;
    }

    println!("Installing update...");

    // Backup current executable
    if local::path_exists(&current_exe) {
        local::copy_file(&current_exe, &backup_path)?;
    }

    // Replace current executable
    local::copy_file(&extracted_binary, &current_exe)?;

    // Clean up temp files
    local::remove_file(&temp_archive).ok();
    local::remove_dir_all(&temp_dir).ok();

    println!("✓ Update installed successfully!");
    println!("  Backup saved to: {}", backup_path.display());
    println!();
    println!("  Please restart the CLI to use the new version.");

    Ok(())
}
