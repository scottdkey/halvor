// GitHub release management
use crate::docker::build::{get_git_hash, get_github_user};
use anyhow::{Context, Result};
use serde_json;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::process::Command;

/// Push CLI binaries to GitHub releases
/// If release_tag is None, creates a development-{hash} tag
/// If release_tag is Some("experimental"), creates/updates the experimental release
pub fn push_cli_to_github(binaries: &[(String, PathBuf)], release_tag: Option<&str>) -> Result<()> {
    let github_user = get_github_user();
    let git_hash = get_git_hash();
    
    let (tag, release_name) = if let Some(tag_name) = release_tag {
        (tag_name.to_string(), format!("Experimental Build ({})", git_hash))
    } else {
        (format!("development-{}", git_hash), format!("Development Build ({})", git_hash))
    };

    // Check for GitHub token
    let github_token = std::env::var("GITHUB_TOKEN")
        .context("GITHUB_TOKEN environment variable is required for pushing releases")?;

    let repo = format!("{}/halvor", github_user);
    let api_url = format!("https://api.github.com/repos/{}/releases", repo);

    println!("Creating GitHub release: {}", tag);

    // Create release body
    let mut release_body = format!(
        "## Development Build\n\nGit commit: `{}`\n\n### Downloads\n\n",
        git_hash
    );

    // Prepare assets - create tarballs for each binary
    let mut assets: Vec<(String, PathBuf, String)> = Vec::new();
    let temp_dir = std::env::temp_dir();
    
    for (target, binary_path) in binaries {
        // Create tarball for this binary
        let tarball_name = format_tarball_name(target);
        let tarball_path = temp_dir.join(&tarball_name);
        
        // Create tarball containing the binary named "halvor"
        create_tarball(binary_path, &tarball_path)?;
        
        assets.push((tarball_name.clone(), tarball_path.clone(), target.clone()));
        release_body.push_str(&format!("- **{}**: `{}`\n", target, tarball_name));
    }

    // Create or update release
    let client = reqwest::blocking::Client::new();

    // Check if release exists
    let release_id = get_or_create_release(
        &client,
        &api_url,
        &tag,
        &release_name,
        &release_body,
        &github_token,
    )?;

    // Upload assets
    upload_assets(&client, &repo, release_id, &assets, &github_token)?;

    println!(
        "\n✓ Release created/updated: https://github.com/{}/releases/tag/{}",
        repo, tag
    );
    Ok(())
}

/// Get existing release ID or create a new release
fn get_or_create_release(
    client: &reqwest::blocking::Client,
    api_url: &str,
    tag: &str,
    release_name: &str,
    release_body: &str,
    github_token: &str,
) -> Result<u64> {
    let check_url = format!("{}/tags/{}", api_url, tag);
    let check_response = client
        .get(&check_url)
        .header("Authorization", format!("token {}", github_token))
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .context("Failed to check for existing release")?;

    if check_response.status() == 200 {
        // Release exists, get its ID
        let release: serde_json::Value = check_response
            .json()
            .context("Failed to parse release response")?;
        let id = release["id"]
            .as_u64()
            .context("Release ID not found in response")?;
        println!("  Release already exists, updating...");
        Ok(id)
    } else {
        // Create new release
        let create_payload = serde_json::json!({
            "tag_name": tag,
            "name": release_name,
            "body": release_body,
            "prerelease": true,
            "draft": false
        });

        let create_response = client
            .post(api_url)
            .header("Authorization", format!("token {}", github_token))
            .header("Accept", "application/vnd.github.v3+json")
            .json(&create_payload)
            .send()
            .context("Failed to create release")?;

        let status = create_response.status();
        if !status.is_success() {
            let error_text = create_response.text().unwrap_or_default();
            anyhow::bail!("Failed to create release: HTTP {} - {}", status, error_text);
        }

        let release: serde_json::Value = create_response
            .json()
            .context("Failed to parse release response")?;
        release["id"]
            .as_u64()
            .context("Release ID not found in response")
    }
}

/// Upload assets to GitHub release
fn upload_assets(
    client: &reqwest::blocking::Client,
    repo: &str,
    release_id: u64,
    assets: &[(String, PathBuf, String)],
    github_token: &str,
) -> Result<()> {
    let upload_url = format!(
        "https://uploads.github.com/repos/{}/releases/{}/assets",
        repo, release_id
    );

    for (asset_name, binary_path, target) in assets {
        println!("  Uploading: {} ({})", asset_name, target);

        let mut file = File::open(binary_path).context("Failed to open binary file")?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .context("Failed to read binary file")?;

        let encoded_name = url_encode_asset_name(asset_name);

        let upload_response = client
            .post(&format!("{}?name={}", upload_url, encoded_name))
            .header("Authorization", format!("token {}", github_token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("Content-Type", "application/octet-stream")
            .header("Content-Length", buffer.len().to_string())
            .body(buffer)
            .send()
            .context(format!("Failed to upload asset: {}", asset_name))?;

        let upload_status = upload_response.status();
        if !upload_status.is_success() {
            let error_text = upload_response.text().unwrap_or_default();
            eprintln!(
                "  ⚠️  Warning: Failed to upload {}: HTTP {} - {}",
                asset_name, upload_status, error_text
            );
        } else {
            println!("  ✓ Uploaded: {}", asset_name);
        }
    }

    Ok(())
}

/// Format asset name for GitHub release
/// Distinguishes between gnu and musl Linux targets
#[allow(dead_code)]
fn format_asset_name(target: &str, extension: &str) -> String {
    // Convert target to a readable format
    // Examples:
    // x86_64-unknown-linux-gnu -> halvor-x86_64-linux-gnu.bin
    // x86_64-unknown-linux-musl -> halvor-x86_64-linux-musl.bin
    // aarch64-apple-darwin -> halvor-aarch64-apple-darwin.bin
    // x86_64-pc-windows-msvc -> halvor-x86_64-windows.exe

    let mut parts: Vec<&str> = target.split('-').collect();

    // Handle Linux targets specially to make gnu vs musl clear
    if target.contains("linux") {
        // Remove "unknown" from the target name for cleaner output
        parts.retain(|&p| p != "unknown");

        // Ensure musl is clearly marked
        let target_clean = parts.join("-");
        format!("halvor-{}.{}", target_clean, extension)
    } else if target.contains("windows") {
        // Remove "pc" and "msvc" for cleaner Windows names
        parts.retain(|&p| p != "pc" && p != "msvc");
        let target_clean = parts.join("-");
        format!("halvor-{}.{}", target_clean, extension)
    } else {
        // For other targets (like apple-darwin), keep as is but remove "unknown" if present
        parts.retain(|&p| p != "unknown");
        let target_clean = parts.join("-");
        format!("halvor-{}.{}", target_clean, extension)
    }
}

/// Create a tarball from a binary file
/// The tarball will contain the binary named "halvor" (or "halvor.exe" for Windows)
fn create_tarball(binary_path: &PathBuf, tarball_path: &PathBuf) -> Result<()> {
    let binary_name = if binary_path.to_string_lossy().contains("windows") {
        "halvor.exe"
    } else {
        "halvor"
    };
    
    // Create tarball: copy binary to temp location with correct name, then tar it
    let temp_binary = std::env::temp_dir().join(binary_name);
    std::fs::copy(binary_path, &temp_binary)
        .context("Failed to copy binary to temp location")?;
    
    // Use tar command to create tarball from temp binary
    let status = Command::new("tar")
        .arg("czf")
        .arg(tarball_path)
        .arg("-C")
        .arg(temp_binary.parent().unwrap())
        .arg(binary_name)
        .status()
        .context("Failed to create tarball")?;
    
    // Clean up temp binary
    let _ = std::fs::remove_file(&temp_binary);
    
    if !status.success() {
        anyhow::bail!("tar command failed with status: {:?}", status.code());
    }
    
    Ok(())
}

/// Format tarball name for GitHub release
/// Matches the format expected by download_halvor_from_github
/// The download function looks for assets containing the platform string
/// Platform format: "linux-amd64", "linux-arm64", "darwin-amd64", "darwin-arm64", etc.
fn format_tarball_name(target: &str) -> String {
    // Convert target to platform format expected by download function
    // The download function constructs platform as: "{os}-{arch}" (e.g., "linux-amd64")
    // Examples:
    // x86_64-unknown-linux-gnu -> halvor-linux-amd64.tar.gz
    // aarch64-unknown-linux-gnu -> halvor-linux-arm64.tar.gz
    // x86_64-unknown-linux-musl -> halvor-linux-amd64.tar.gz (musl handled separately)
    // aarch64-apple-darwin -> halvor-darwin-arm64.tar.gz
    // x86_64-apple-darwin -> halvor-darwin-amd64.tar.gz
    
    let platform = if target.contains("linux") {
        let arch = if target.contains("aarch64") || target.contains("arm64") {
            "arm64"
        } else {
            "amd64"
        };
        format!("linux-{}", arch)
    } else if target.contains("darwin") || target.contains("apple") {
        let arch = if target.contains("aarch64") || target.contains("arm64") {
            "arm64"
        } else {
            "amd64"
        };
        format!("darwin-{}", arch)
    } else if target.contains("windows") {
        let arch = if target.contains("aarch64") || target.contains("arm64") {
            "arm64"
        } else {
            "amd64"
        };
        format!("windows-{}", arch)
    } else {
        // Fallback: use target as-is but clean it up
        target.replace("unknown-", "").replace("pc-", "")
    };
    
    format!("halvor-{}.tar.gz", platform)
}

/// URL encode asset name for GitHub API
fn url_encode_asset_name(name: &str) -> String {
    name.replace(" ", "%20")
        .replace("!", "%21")
        .replace("#", "%23")
        .replace("$", "%24")
        .replace("%", "%25")
        .replace("&", "%26")
        .replace("'", "%27")
        .replace("(", "%28")
        .replace(")", "%29")
        .replace("*", "%2A")
        .replace("+", "%2B")
        .replace(",", "%2C")
        .replace(":", "%3A")
        .replace(";", "%3B")
        .replace("=", "%3D")
        .replace("?", "%3F")
        .replace("@", "%40")
        .replace("[", "%5B")
        .replace("]", "%5D")
}

