// App Store Connect integration
use crate::common::{ensure_dir_exists, ensure_path_exists, execute_command};
use halvor_core::utils::env;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

/// Push iOS app to App Store Connect
pub fn push_ios_to_app_store() -> Result<()> {
    push_ios_to_app_store_impl()
}

/// Push iOS app to App Store Connect (implementation)
pub fn push_ios_to_app_store_impl() -> Result<()> {
    println!("Pushing iOS app to App Store Connect...");

    // First, ensure the app was built and signed
    let app_path =
        PathBuf::from("projects/ios/build/Build/Products/Release-iphoneos/HalvorApp-iOS.app");
    ensure_path_exists(
        &app_path,
        &format!(
            "iOS app not found at {}. Build step must have failed.",
            app_path.display()
        ),
    )?;

    // Create IPA archive
    let ipa_path_abs = create_ipa_archive(&app_path)?;
    println!("✓ IPA archive created: {}", ipa_path_abs.display());

    // Upload to App Store Connect using Fastlane
    println!("Uploading to App Store Connect via Fastlane...");
    let ipa_path_str = ipa_path_abs
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("IPA path contains invalid UTF-8"))?;
    let mut fastlane_cmd = prepare_fastlane_command(ipa_path_str)?;
    configure_app_store_credentials(&mut fastlane_cmd)?;

    execute_command(
        fastlane_cmd,
        "Failed to upload to App Store Connect. Check Fastlane output for details.",
    )?;

    println!("✓ iOS app uploaded to App Store Connect successfully");
    println!("  IPA location: {}", ipa_path_abs.display());
    println!("  Check App Store Connect for processing status.");

    Ok(())
}

/// Create IPA archive from app bundle
fn create_ipa_archive(app_path: &PathBuf) -> Result<PathBuf> {
    println!("Creating IPA archive...");
    let ipa_dir = PathBuf::from("projects/ios/build/ipa");
    ensure_dir_exists(&ipa_dir)?;

    let payload_dir = ipa_dir.join("Payload");
    ensure_dir_exists(&payload_dir)?;

    // Copy app bundle to Payload directory
    let dest_app = payload_dir.join("HalvorApp-iOS.app");
    if dest_app.exists() {
        std::fs::remove_dir_all(&dest_app).context("Failed to remove existing app in Payload")?;
    }

    // Use ditto to copy the app bundle (preserves symlinks and metadata)
    let status = Command::new("ditto")
        .args([app_path, &dest_app])
        .status()
        .context("Failed to copy app bundle to Payload")?;

    if !status.success() {
        anyhow::bail!("Failed to copy app bundle to Payload directory");
    }

    // Create IPA file
    let ipa_path = ipa_dir.join("HalvorApp-iOS.ipa");
    if ipa_path.exists() {
        std::fs::remove_file(&ipa_path).with_context(|| {
            format!("Failed to remove existing IPA file: {}", ipa_path.display())
        })?;
    }

    // Use absolute path for the IPA file to avoid issues with zip
    let ipa_path_abs = std::fs::canonicalize(&ipa_dir)
        .context("Failed to get absolute path for IPA directory")?
        .join("HalvorApp-iOS.ipa");

    let ipa_path_str = ipa_path_abs
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("IPA path contains invalid UTF-8"))?;

    let status = Command::new("zip")
        .args(["-r", ipa_path_str, "Payload"])
        .current_dir(&ipa_dir)
        .status()
        .with_context(|| {
            format!(
                "Failed to create IPA archive at: {}",
                ipa_path_abs.display()
            )
        })?;

    if !status.success() {
        anyhow::bail!("Failed to create IPA archive");
    }

    Ok(ipa_path_abs)
}

/// Prepare Fastlane command with direnv environment
fn prepare_fastlane_command(ipa_path_str: &str) -> Result<Command> {
    let base_dir = PathBuf::from(".");
    let mut fastlane_cmd = env::shell_command_with_direnv(
        &base_dir,
        "cd fastlane && fastlane ios ios_upload_to_app_store",
        Some(&base_dir),
    );
    fastlane_cmd.env("IPA_PATH", ipa_path_str);
    Ok(fastlane_cmd)
}

/// Configure App Store Connect credentials from environment
fn configure_app_store_credentials(cmd: &mut Command) -> Result<()> {
    eprintln!("Checking for App Store Connect credentials in environment...");
    let mut found_vars = Vec::new();
    let mut missing_vars = Vec::new();

    // Handle API key path (may be from 1Password)
    let api_key_path = get_api_key_path()?;
    if let Some(ref key_path) = api_key_path {
        cmd.env("APP_STORE_CONNECT_API_KEY_PATH", key_path);
        found_vars.push("APP_STORE_CONNECT_API_KEY_PATH");
        eprintln!("  ✓ Found: APP_STORE_CONNECT_API_KEY_PATH");
    } else {
        missing_vars.push("APP_STORE_CONNECT_API_KEY_PATH");
        eprintln!("  ✗ Missing: APP_STORE_CONNECT_API_KEY_PATH");
    }

    // Check other credential variables
    let credential_vars = [
        "APP_STORE_CONNECT_API_KEY_ID",
        "APP_STORE_CONNECT_API_ISSUER",
        "FASTLANE_TEAM_ID",
        "APP_STORE_CONNECT_TEAM_ID",
        "APP_STORE_CONNECT_TEAM",
        "FASTLANE_USER",
        "APP_STORE_CONNECT_USERNAME",
        "FASTLANE_PASSWORD",
        "APP_STORE_CONNECT_PASSWORD",
    ];

    for var_name in &credential_vars {
        match std::env::var(var_name) {
            Ok(val) => {
                if !val.is_empty() {
                    cmd.env(var_name, &val);
                    found_vars.push(*var_name);
                    println!("  ✓ Found: {}", var_name);
                } else {
                    missing_vars.push(*var_name);
                    println!("  ✗ Empty: {}", var_name);
                }
            }
            Err(_) => {
                missing_vars.push(*var_name);
                println!("  ✗ Missing: {}", var_name);
            }
        }
    }

    // Print summary
    eprintln!();
    if found_vars.len() >= 3 {
        eprintln!("✓ All required API key credentials found!");
    } else if !found_vars.is_empty() {
        eprintln!(
            "⚠️  Found {} credential(s), but need 3 for API key auth or 2 for username/password",
            found_vars.len()
        );
        eprintln!("   Found: {}", found_vars.join(", "));
        eprintln!("   Missing: {}", missing_vars.join(", "));
    } else {
        eprintln!("⚠️  No App Store Connect credentials found in environment!");
        eprintln!("   Make sure:");
        eprintln!("   1. Variables are in your 1Password vault with exact names");
        eprintln!("   2. direnv is loaded (run 'direnv allow' in this directory)");
        eprintln!("   3. You're running halvor from a shell where direnv is active");
    }
    eprintln!();

    Ok(())
}

/// Get API key path, downloading from 1Password if needed
fn get_api_key_path() -> Result<Option<String>> {
    match std::env::var("APP_STORE_CONNECT_API_KEY_PATH") {
        Ok(path) if !path.is_empty() => {
            // Check if it's a 1Password reference (op://)
            if path.starts_with("op://") {
                println!("  → Downloading API key from 1Password...");
                let temp_key_path = std::env::temp_dir().join("app_store_connect_api_key.p8");

                let status = Command::new("op")
                    .args(["read", &path, "--out-file", temp_key_path.to_str().unwrap()])
                    .status()
                    .context("Failed to download API key from 1Password")?;

                if !status.success() {
                    anyhow::bail!(
                        "Failed to download API key from 1Password. Make sure you're signed in: op signin"
                    );
                }

                println!("  ✓ Downloaded API key to temporary file");
                Ok(Some(temp_key_path.to_string_lossy().to_string()))
            } else if std::path::Path::new(&path).exists() {
                Ok(Some(path))
            } else {
                println!("  ⚠️  API key path doesn't exist: {}", path);
                Ok(None)
            }
        }
        _ => {
            // Try to download from 1Password using the helper function
            eprintln!(
                "  → APP_STORE_CONNECT_API_KEY_PATH not set, attempting to download from 1Password..."
            );
            download_api_key_from_1password()
        }
    }
}

/// Download API key from 1Password
fn download_api_key_from_1password() -> Result<Option<String>> {
    let vault = std::env::var("VAULT_NAME").unwrap_or_else(|_| "automations".to_string());
    let item = std::env::var("ITEM_NAME").unwrap_or_else(|_| "halvor".to_string());

    eprintln!(
        "  → Attempting to download API key from 1Password item '{}' in vault '{}'...",
        item, vault
    );

    // First, get the item JSON to find .p8 files
    let item_output = Command::new("op")
        .args(["item", "get", &item, "--vault", &vault, "--format", "json"])
        .output()
        .context("Failed to query 1Password item")?;

    if !item_output.status.success() {
        eprintln!("  ⚠️  Could not access 1Password item. Make sure you're signed in: op signin");
        return Ok(None);
    }

    // Parse JSON to find .p8 files
    let item_json: serde_json::Value = serde_json::from_slice(&item_output.stdout)
        .context("Failed to parse 1Password item JSON")?;

    // Look for files in the item
    let mut p8_files = Vec::new();
    if let Some(files) = item_json.get("files").and_then(|f| f.as_array()) {
        for file in files {
            if let Some(name) = file.get("name").and_then(|n| n.as_str()) {
                if name.ends_with(".p8") {
                    p8_files.push(name.to_string());
                }
            }
        }
    }

    if p8_files.is_empty() {
        eprintln!("  ⚠️  No .p8 files found in 1Password item");
        return Ok(None);
    }

    // Use the first .p8 file found (or prefer AuthKey.p8 if it exists)
    let file_name = if p8_files.iter().any(|f| f == "AuthKey.p8") {
        "AuthKey.p8"
    } else {
        &p8_files[0]
    };

    eprintln!("  → Found .p8 file: {}", file_name);
    let temp_key_path = std::env::temp_dir().join("app_store_connect_api_key.p8");

    // Use op:// reference format to download the file
    let file_ref = format!("op://{}/{}/{}", vault, item, file_name);
    eprintln!("  → Downloading file using reference: {}", file_ref);

    let output = Command::new("op")
        .args([
            "read",
            &file_ref,
            "--out-file",
            temp_key_path.to_str().unwrap(),
        ])
        .output()
        .context("Failed to download API key from 1Password")?;

    if output.status.success() && temp_key_path.exists() {
        eprintln!("  ✓ Downloaded API key file '{}' from 1Password", file_name);
        Ok(Some(temp_key_path.to_string_lossy().to_string()))
    } else {
        eprintln!("  ⚠️  Failed to download API key file from 1Password");
        Ok(None)
    }
}

