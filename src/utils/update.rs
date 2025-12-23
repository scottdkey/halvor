use crate::utils::exec::local;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::env;
use std::io::Write;

const GITHUB_API_BASE: &str = "https://api.github.com";
const REPO_OWNER: &str = "scottdkey"; // TODO: Make this configurable
const REPO_NAME: &str = "halvor";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseChannel {
    Experimental,
    Stable,
    Unknown,
}

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    prerelease: bool,
    #[serde(default)]
    published_at: Option<String>,
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
        .context("Failed to fetch releases")?;

    if !response.status().is_success() {
        // Silently fail - network issues shouldn't block the CLI
        return Ok(None);
    }

    let release: Release = response.json().context("Failed to parse release JSON")?;
    // Only check non-prerelease releases
    if release.prerelease {
        return Ok(None);
    }
    let latest_version = release.tag_name.trim_start_matches('v');
    let current_version_normalized = current_version.trim_start_matches('v');
    if latest_version != current_version_normalized && latest_version > current_version_normalized {
        return Ok(Some(release.tag_name));
    }

    Ok(None)
}

pub fn check_for_experimental_updates(_current_version: &str) -> Result<Option<String>> {
    // Skip update check in development mode
    if env::var("HAL_DEV_MODE").is_ok() || cfg!(debug_assertions) {
        return Ok(None);
    }

    let client = reqwest::blocking::Client::builder()
        .user_agent("hal-cli")
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .context("Failed to create HTTP client")?;

    // Get the experimental release (tagged as "experimental")
    let url = format!(
        "{}/repos/{}/{}/releases/tags/experimental",
        GITHUB_API_BASE, REPO_OWNER, REPO_NAME
    );

    let response = client
        .get(&url)
        .send()
        .context("Failed to fetch experimental release")?;

    if !response.status().is_success() {
        // Silently fail - experimental release may not exist yet
        return Ok(None);
    }

    let release: Release = response.json().context("Failed to parse release JSON")?;

    // For experimental, check if the release is newer than the current executable
    // by comparing timestamps
    if let Some(published_at_str) = &release.published_at {
        // Parse the published_at timestamp (ISO 8601 format)
        let published_at = chrono::DateTime::parse_from_rfc3339(published_at_str)
            .context("Failed to parse release timestamp")?;

        // Get the current executable's modification time
        let current_exe = env::current_exe().context("Failed to get current executable path")?;
        let metadata =
            std::fs::metadata(&current_exe).context("Failed to get executable metadata")?;

        #[cfg(unix)]
        let exe_mtime = {
            use std::os::unix::fs::MetadataExt;
            std::time::SystemTime::UNIX_EPOCH
                + std::time::Duration::from_secs(metadata.mtime() as u64)
        };

        #[cfg(windows)]
        let exe_mtime = metadata
            .modified()
            .context("Failed to get executable modification time")?;

        // Convert SystemTime to DateTime<Utc>
        let exe_datetime: chrono::DateTime<chrono::Utc> = exe_mtime.into();
        let published_datetime: chrono::DateTime<chrono::Utc> = published_at.into();

        // Only return experimental if the release is newer than the current executable
        if published_datetime > exe_datetime {
            return Ok(Some("experimental".to_string()));
        }
    } else {
        // If no timestamp available, assume it's newer (fallback behavior)
        return Ok(Some("experimental".to_string()));
    }

    Ok(None)
}

/// Detect which release channel the current binary is from
/// by comparing the executable's modification time with release timestamps
pub fn detect_release_channel() -> Result<ReleaseChannel> {
    // Skip check in development mode
    if env::var("HAL_DEV_MODE").is_ok() || cfg!(debug_assertions) {
        return Ok(ReleaseChannel::Unknown);
    }

    let current_exe = env::current_exe().context("Failed to get current executable path")?;
    let metadata = std::fs::metadata(&current_exe).context("Failed to get executable metadata")?;

    #[cfg(unix)]
    let exe_mtime = {
        use std::os::unix::fs::MetadataExt;
        std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(metadata.mtime() as u64)
    };

    #[cfg(windows)]
    let exe_mtime = metadata
        .modified()
        .context("Failed to get executable modification time")?;

    let exe_datetime: chrono::DateTime<chrono::Utc> = exe_mtime.into();

    // Check experimental release timestamp
    let client = reqwest::blocking::Client::builder()
        .user_agent("hal-cli")
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .context("Failed to create HTTP client")?;

    let experimental_url = format!(
        "{}/repos/{}/{}/releases/tags/experimental",
        GITHUB_API_BASE, REPO_OWNER, REPO_NAME
    );

    if let Ok(response) = client.get(&experimental_url).send() {
        if response.status().is_success() {
            if let Ok(release) = response.json::<Release>() {
                if let Some(published_at_str) = &release.published_at {
                    if let Ok(published_at) = chrono::DateTime::parse_from_rfc3339(published_at_str)
                    {
                        let published_datetime: chrono::DateTime<chrono::Utc> = published_at.into();
                        // If executable time is within 1 hour of experimental release time, it's likely experimental
                        let time_diff = (exe_datetime - published_datetime).num_seconds().abs();
                        if time_diff < 3600 {
                            return Ok(ReleaseChannel::Experimental);
                        }
                    }
                }
            }
        }
    }

    // Check stable releases
    let latest_url = format!(
        "{}/repos/{}/{}/releases/latest",
        GITHUB_API_BASE, REPO_OWNER, REPO_NAME
    );

    if let Ok(response) = client.get(&latest_url).send() {
        if response.status().is_success() {
            if let Ok(release) = response.json::<Release>() {
                if !release.prerelease {
                    if let Some(published_at_str) = &release.published_at {
                        if let Ok(published_at) =
                            chrono::DateTime::parse_from_rfc3339(published_at_str)
                        {
                            let published_datetime: chrono::DateTime<chrono::Utc> =
                                published_at.into();
                            // If executable time is within 1 hour of stable release time, it's likely stable
                            let time_diff = (exe_datetime - published_datetime).num_seconds().abs();
                            if time_diff < 3600 {
                                return Ok(ReleaseChannel::Stable);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(ReleaseChannel::Unknown)
}

pub fn get_latest_version() -> Result<String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("hal-cli")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .context("Failed to create HTTP client")?;

    let url = format!(
        "{}/repos/{}/{}/releases/latest",
        GITHUB_API_BASE, REPO_OWNER, REPO_NAME
    );

    let response = client
        .get(&url)
        .send()
        .context("Failed to fetch releases")?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to fetch releases: HTTP {}", response.status());
    }

    let release: Release = response.json().context("Failed to parse release JSON")?;
    // Only return non-prerelease releases
    if release.prerelease {
        anyhow::bail!("No stable release found (only prereleases available)");
    }
    Ok(release.tag_name)
}

pub fn get_latest_experimental_version() -> Result<String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("hal-cli")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .context("Failed to create HTTP client")?;

    // Get the experimental release (tagged as "experimental")
    let url = format!(
        "{}/repos/{}/{}/releases/tags/experimental",
        GITHUB_API_BASE, REPO_OWNER, REPO_NAME
    );

    let response = client
        .get(&url)
        .send()
        .context("Failed to fetch experimental release")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to fetch experimental release: HTTP {}",
            response.status()
        );
    }

    let _release: Release = response.json().context("Failed to parse release JSON")?;
    Ok("experimental".to_string())
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
    // For experimental releases, version is "experimental"
    let extension = if cfg!(target_os = "windows") {
        ".zip"
    } else {
        ".tar.gz"
    };

    // For experimental, we need to find the asset by pattern since version is "experimental"
    let version_clean = version.trim_start_matches('v');
    let asset_name = if version_clean == "experimental" {
        // For experimental, we'll search for matching assets
        format!("hal-*-{}-{}{}", platform, arch, extension)
    } else {
        format!("hal-{}-{}-{}{}", version_clean, platform, arch, extension)
    };

    let download_url = if version_clean == "experimental" {
        // For experimental, we need to fetch release assets and find matching one
        format!(
            "https://github.com/{}/{}/releases/download/{}/{}",
            REPO_OWNER, REPO_NAME, version, asset_name
        )
    } else {
        format!(
            "https://github.com/{}/{}/releases/download/{}/{}",
            REPO_OWNER, REPO_NAME, version, asset_name
        )
    };

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
        // If the specific asset doesn't exist, try to get the release assets list
        // and find a matching one
        if response.status() == 404 {
            println!("Expected asset not found, searching release assets...");
            // Try to fetch release assets and find a matching one
            let release_url = format!(
                "{}/repos/{}/{}/releases/tags/{}",
                GITHUB_API_BASE, REPO_OWNER, REPO_NAME, version
            );
            let release_response = client
                .get(&release_url)
                .send()
                .context("Failed to fetch release info")?;

            if release_response.status().is_success() {
                #[derive(Deserialize)]
                struct ReleaseInfo {
                    assets: Vec<AssetInfo>,
                }
                #[derive(Deserialize)]
                struct AssetInfo {
                    name: String,
                    browser_download_url: String,
                }

                let release_info: ReleaseInfo = release_response
                    .json()
                    .context("Failed to parse release info")?;

                if release_info.assets.is_empty() {
                    anyhow::bail!(
                        "Release {} exists but has no assets. Please create a release with build artifacts.",
                        version
                    );
                }

                // Try to find a matching asset
                // For experimental, match by platform and arch pattern
                let matching_asset =
                    if version.trim_start_matches('v') == "experimental" {
                        release_info.assets.iter().find(|asset| {
                            asset.name.contains(&platform) && asset.name.contains(&arch)
                        })
                    } else {
                        release_info.assets.iter().find(|asset| {
                            asset.name.contains(&platform) && asset.name.contains(&arch)
                        })
                    };

                if let Some(asset) = matching_asset {
                    println!("Found matching asset: {}", asset.name);
                    // Use the found asset URL
                    return download_and_install_from_url(&asset.browser_download_url, version);
                } else {
                    // Show available assets for debugging
                    eprintln!(
                        "No matching asset found for platform '{}' and arch '{}'",
                        platform, arch
                    );
                    eprintln!("Available assets:");
                    for asset in &release_info.assets {
                        eprintln!("  - {}", asset.name);
                    }
                    anyhow::bail!(
                        "No matching asset found for this platform ({}) and architecture ({})",
                        platform,
                        arch
                    );
                }
            } else if release_response.status() == 404 {
                anyhow::bail!(
                    "Release {} not found. The release may not exist yet or may be a draft.",
                    version
                );
            } else {
                anyhow::bail!(
                    "Failed to fetch release info: HTTP {}",
                    release_response.status()
                );
            }
        }
        anyhow::bail!("Failed to download update: HTTP {}", response.status());
    }

    let temp_archive = std::env::temp_dir().join(format!("hal-update-{}{}", version, extension));
    let mut file = std::fs::File::create(&temp_archive).context("Failed to create temp file")?;
    std::io::copy(&mut response.bytes()?.as_ref(), &mut file)
        .context("Failed to write download")?;
    drop(file);

    // Continue with extraction and installation
    extract_and_install(&temp_archive, &current_exe, &backup_path, version)
}

fn extract_and_install(
    temp_archive: &std::path::Path,
    current_exe: &std::path::Path,
    backup_path: &std::path::Path,
    version: &str,
) -> Result<()> {
    println!("Extracting archive...");

    // Extract the archive
    let temp_dir = std::env::temp_dir().join(format!("hal-update-extract-{}", version));
    local::create_dir_all(&temp_dir)?;

    let extracted_binary: std::path::PathBuf = if cfg!(target_os = "windows") {
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
    // On Linux, we can't overwrite a file that's being executed, so we need to:
    // 1. Copy to a temporary location next to the target
    // 2. Remove the old file
    // 3. Rename the new file to the target (atomic operation)
    #[cfg(unix)]
    {
        let temp_target = current_exe.with_extension("hal.new");
        std::fs::copy(&extracted_binary, &temp_target)
            .context("Failed to copy new binary to temp location")?;
        local::set_permissions(&temp_target, 0o755)
            .context("Failed to set permissions on new binary")?;

        // Remove the old file (this works even if it's being executed)
        std::fs::remove_file(&current_exe).context("Failed to remove old binary")?;

        // Atomically rename the new file to the target
        std::fs::rename(&temp_target, &current_exe)
            .context("Failed to rename new binary to target location")?;

        // Remove backup after successful rename
        if local::path_exists(&backup_path) {
            local::remove_file(&backup_path).context("Failed to remove backup file")?;
        }
    }

    #[cfg(windows)]
    {
        // On Windows, we can overwrite directly
        local::copy_file(&extracted_binary, &current_exe)?;

        // Remove backup after successful copy
        if local::path_exists(&backup_path) {
            local::remove_file(&backup_path).context("Failed to remove backup file")?;
        }
    }

    // Clean up temp files
    local::remove_file(&temp_archive).ok();
    local::remove_dir_all(&temp_dir).ok();

    println!("✓ Update installed successfully!");
    println!();
    println!("  Please restart the CLI to use the new version.");

    Ok(())
}

/// Helper function to download and install from a specific URL
fn download_and_install_from_url(download_url: &str, version: &str) -> Result<()> {
    println!("Downloading from: {}", download_url);

    // Get current executable path
    let current_exe = env::current_exe().context("Failed to get current executable path")?;
    let extension = if cfg!(target_os = "windows") {
        ".zip"
    } else {
        ".tar.gz"
    };
    let backup_path = current_exe.with_extension(format!("{}.bak", extension));

    // Download to temp file
    let client = reqwest::blocking::Client::builder()
        .user_agent("hal-cli")
        .build()
        .context("Failed to create HTTP client")?;

    let response = client
        .get(download_url)
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

    // Continue with extraction and installation
    extract_and_install(&temp_archive, &current_exe, &backup_path, version)
}
