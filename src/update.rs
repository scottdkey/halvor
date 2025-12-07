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
    assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
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

    // Compare versions (simple string comparison, assumes semver)
    if release.tag_name != current_version && release.tag_name.as_str() > current_version {
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
    let (platform, extension) = if cfg!(target_os = "linux") {
        ("linux", "")
    } else if cfg!(target_os = "macos") {
        ("macos", "")
    } else if cfg!(target_os = "windows") {
        ("windows", ".exe")
    } else {
        anyhow::bail!("Unsupported platform for auto-update");
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        anyhow::bail!("Unsupported architecture for auto-update");
    };

    let asset_name = format!("hal-{}-{}{}", platform, arch, extension);
    let download_url = format!(
        "https://github.com/{}/{}/releases/download/{}/{}",
        REPO_OWNER, REPO_NAME, version, asset_name
    );

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

    let temp_file = std::env::temp_dir().join(format!("hal-update-{}", version));
    let mut file = std::fs::File::create(&temp_file).context("Failed to create temp file")?;
    std::io::copy(&mut response.bytes()?.as_ref(), &mut file)
        .context("Failed to write download")?;
    drop(file);

    // Make executable (Unix)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temp_file, std::fs::Permissions::from_mode(0o755))
            .context("Failed to set executable permissions")?;
    }

    println!("Installing update...");

    // Backup current executable
    if current_exe.exists() {
        std::fs::copy(&current_exe, &backup_path).context("Failed to backup current executable")?;
    }

    // Replace current executable
    std::fs::copy(&temp_file, &current_exe).context("Failed to install update")?;

    // Clean up temp file
    std::fs::remove_file(&temp_file).ok();

    println!("✓ Update installed successfully!");
    println!("  Backup saved to: {}", backup_path.display());
    println!();
    println!("  Please restart the CLI to use the new version.");

    Ok(())
}
