// iOS and macOS build and signing
use crate::common::ensure_path_exists;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

/// Build iOS app using Fastlane
pub fn build_ios() -> Result<()> {
    println!("Building iOS app with Fastlane...");

    let status = Command::new("fastlane")
        .args(["ios", "ios_build_app"])
        .current_dir("fastlane")
        .status()
        .context("Failed to execute Fastlane iOS build")?;

    if !status.success() {
        anyhow::bail!("iOS build failed. Check Fastlane output for details.");
    }

    Ok(())
}

/// Build macOS app using Fastlane
pub fn build_mac() -> Result<()> {
    println!("Building macOS app with Fastlane...");

    let status = Command::new("fastlane")
        .args(["mac", "mac_build_app"])
        .current_dir("fastlane")
        .status()
        .context("Failed to execute Fastlane macOS build")?;

    if !status.success() {
        anyhow::bail!("macOS build failed. Check Fastlane output for details.");
    }

    Ok(())
}

/// Sign iOS app using Fastlane
pub fn sign_ios() -> Result<()> {
    println!("Signing iOS app with Fastlane...");

    let app_path =
        PathBuf::from("projects/ios/build/Build/Products/Release-iphoneos/HalvorApp-iOS.app");
    ensure_path_exists(
        &app_path,
        &format!(
            "iOS app not found at {}. Build step must have failed.",
            app_path.display()
        ),
    )?;

    let status = Command::new("fastlane")
        .args(["ios", "sign_app"])
        .current_dir("fastlane")
        .status()
        .context("Failed to execute Fastlane iOS signing")?;

    if !status.success() {
        anyhow::bail!("iOS signing failed. Check Fastlane output for details.");
    }

    println!("✓ iOS app signed successfully");
    Ok(())
}

/// Sign macOS app using Fastlane
pub fn sign_mac() -> Result<()> {
    println!("Signing macOS app with Fastlane...");

    let app_path = PathBuf::from("projects/ios/build/Build/Products/Release/HalvorApp-macOS.app");
    ensure_path_exists(
        &app_path,
        &format!(
            "macOS app not found at {}. Build step must have failed.",
            app_path.display()
        ),
    )?;

    let status = Command::new("fastlane")
        .args(["mac", "sign_app"])
        .current_dir("fastlane")
        .status()
        .context("Failed to execute Fastlane macOS signing")?;

    if !status.success() {
        anyhow::bail!("macOS signing failed. Check Fastlane output for details.");
    }

    println!("✓ macOS app signed successfully");
    Ok(())
}

/// Build and sign iOS app
pub fn build_and_sign_ios() -> Result<()> {
    // Generate API client library
    println!("Generating Swift API client library...");
    let workspace_root = std::env::current_dir()?;
    halvor_web::client_gen::generate_all_clients(&workspace_root)?;
    println!("✓ Swift API client library generated");
    
    build_ios()?;
    sign_ios()?;
    Ok(())
}

/// Build and sign macOS app
pub fn build_and_sign_mac() -> Result<()> {
    // Generate API client library
    println!("Generating Swift API client library...");
    let workspace_root = std::env::current_dir()?;
    halvor_web::client_gen::generate_all_clients(&workspace_root)?;
    println!("✓ Swift API client library generated");
    
    build_mac()?;
    sign_mac()?;
    Ok(())
}

