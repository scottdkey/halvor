// Apple platform development (macOS and iOS)
use crate::common::execute_command;
use anyhow::{Context, Result};
use serde_json;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

/// Start macOS development mode
pub fn dev_mac() -> Result<()> {
    println!("Starting macOS development mode...");

    let swift_dir = PathBuf::from("projects/ios");
    let xcode_proj = swift_dir.join("HalvorApp.xcodeproj");

    // Create Xcode project if it doesn't exist
    if !xcode_proj.exists() {
        println!("Xcode project not found. Creating it...");
        let create_script = swift_dir.join("scripts/create-xcode-project.sh");
        if create_script.exists() {
            let status = Command::new("bash")
                .arg(&create_script)
                .current_dir(&swift_dir)
                .status()
                .context("Failed to create Xcode project")?;

            if !status.success() {
                println!("⚠️  Failed to create Xcode project");
            }
        }
    }

    // Build the app (disable signing for dev builds)
    let mut build_cmd = Command::new("xcodebuild");
    build_cmd
        .args([
            "-project",
            "HalvorApp.xcodeproj",
            "-scheme",
            "HalvorApp-macOS",
            "-configuration",
            "Debug",
            "-derivedDataPath",
            "build",
            "CODE_SIGN_IDENTITY=",
            "CODE_SIGNING_REQUIRED=NO",
            "CODE_SIGNING_ALLOWED=NO",
        ])
        .current_dir(&swift_dir);

    execute_command(build_cmd, "macOS build failed")?;

    // Open the app
    let app_path = swift_dir.join("build/Build/Products/Debug/HalvorApp-macOS.app");
    if app_path.exists() {
        Command::new("open")
            .arg(&app_path)
            .status()
            .context("Failed to open macOS app")?;
    }

    Ok(())
}

/// Start iOS development mode
pub fn dev_ios() -> Result<()> {
    println!("Starting iOS development mode...");

    let swift_dir = PathBuf::from("projects/ios");
    let xcode_proj = swift_dir.join("HalvorApp.xcodeproj");

    // Create Xcode project if it doesn't exist
    if !xcode_proj.exists() {
        println!("Xcode project not found. Creating it...");
        let create_script = swift_dir.join("scripts/create-xcode-project.sh");
        if create_script.exists() {
            let status = Command::new("bash")
                .arg(&create_script)
                .current_dir(&swift_dir)
                .status()
                .context("Failed to create Xcode project")?;

            if !status.success() {
                println!("⚠️  Failed to create Xcode project");
            }
        }
    }

    // List available devices and let user choose
    let devices = list_available_devices()?;

    if devices.is_empty() {
        anyhow::bail!(
            "No iOS devices or simulators found. Please create a simulator or connect a device."
        );
    }

    println!("\nAvailable iOS devices:");
    for (index, device) in devices.iter().enumerate() {
        let status = if device.booted { " (booted)" } else { "" };
        println!(
            "  {}. {} - {} ({}){}",
            index + 1,
            device.name,
            device.runtime,
            device.id,
            status
        );
    }

    print!("\nSelect device (1-{}): ", devices.len());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let selection: usize = input
        .trim()
        .parse()
        .context("Invalid selection. Please enter a number.")?;

    if selection < 1 || selection > devices.len() {
        anyhow::bail!(
            "Invalid selection. Please choose a number between 1 and {}",
            devices.len()
        );
    }

    let selected_device = &devices[selection - 1];
    let sim_id = &selected_device.id;

    println!("Using device: {} ({})", selected_device.name, sim_id);

    // Build the app (disable signing for simulator builds)
    let mut build_cmd = Command::new("xcodebuild");
    build_cmd
        .args([
            "-project",
            "HalvorApp.xcodeproj",
            "-scheme",
            "HalvorApp-iOS",
            "-configuration",
            "Debug",
            "-sdk",
            "iphonesimulator",
            "-derivedDataPath",
            "build",
            "-destination",
            &format!("id={}", sim_id),
            "CODE_SIGN_IDENTITY=",
            "CODE_SIGNING_REQUIRED=NO",
            "CODE_SIGNING_ALLOWED=NO",
        ])
        .current_dir(&swift_dir);

    execute_command(build_cmd, "iOS build failed")?;

    // Boot simulator
    Command::new("xcrun")
        .args(["simctl", "boot", sim_id])
        .status()
        .ok(); // Ignore errors (might already be booted)

    // Install and launch app
    let app_path = swift_dir.join("build/Build/Products/Debug-iphonesimulator/HalvorApp-iOS.app");
    if app_path.exists() {
        let status = Command::new("xcrun")
            .args(["simctl", "install", sim_id, app_path.to_str().unwrap()])
            .status()
            .context("Failed to install iOS app")?;

        if !status.success() {
            anyhow::bail!("Failed to install iOS app");
        }

        // Launch app
        let status = Command::new("xcrun")
            .args(["simctl", "launch", sim_id, "dev.scottkey.halvor.ios"])
            .status()
            .context("Failed to launch iOS app")?;

        if !status.success() {
            anyhow::bail!("Failed to launch iOS app");
        }
    }

    Ok(())
}

#[derive(Debug)]
struct Device {
    id: String,
    name: String,
    runtime: String,
    booted: bool,
}

/// List available iOS devices and simulators
fn list_available_devices() -> Result<Vec<Device>> {
    // Get list of all devices (including physical devices)
    let output = Command::new("xcrun")
        .args(["simctl", "list", "devices", "available", "--json"])
        .output()
        .context("Failed to list devices")?;

    if !output.status.success() {
        // Fallback to non-JSON output
        return list_available_devices_legacy();
    }

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("Failed to parse device list JSON")?;

    let mut devices = Vec::new();

    // Parse JSON structure: { "devices": { "iOS 18.0": [...], ... } }
    if let Some(devices_obj) = json.get("devices").and_then(|d| d.as_object()) {
        for (runtime, device_list) in devices_obj {
            if let Some(device_array) = device_list.as_array() {
                for device in device_array {
                    if let Some(id) = device.get("udid").and_then(|u| u.as_str()) {
                        if let Some(name) = device.get("name").and_then(|n| n.as_str()) {
                            let state = device
                                .get("state")
                                .and_then(|s| s.as_str())
                                .unwrap_or("Shutdown");
                            let booted = state == "Booted";

                            devices.push(Device {
                                id: id.to_string(),
                                name: name.to_string(),
                                runtime: runtime.clone(),
                                booted,
                            });
                        }
                    }
                }
            }
        }
    }

    // Sort devices: booted first, then by name
    devices.sort_by(|a, b| match (a.booted, b.booted) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    Ok(devices)
}

/// Fallback device listing using text parsing
fn list_available_devices_legacy() -> Result<Vec<Device>> {
    let output = Command::new("xcrun")
        .args(["simctl", "list", "devices", "available"])
        .output()
        .context("Failed to list devices")?;

    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut devices = Vec::new();
    let mut current_runtime = String::new();

    for line in output_str.lines() {
        // Check if this is a runtime header (e.g., "-- iOS 18.0 --")
        if line.contains("--") && line.contains("iOS") {
            if let Some(start) = line.find("iOS") {
                if let Some(end) = line[start..].find("--") {
                    current_runtime = line[start..start + end].trim().to_string();
                }
            }
        } else if line.contains("(") && line.contains(")") {
            // Parse device line: "    iPhone 17 (XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX) (Booted)"
            let trimmed = line.trim();
            if let Some(name_end) = trimmed.find('(') {
                let name = trimmed[..name_end].trim().to_string();

                if let Some(id_start) = trimmed[name_end..].find('(') {
                    let id_part = &trimmed[name_end + id_start + 1..];
                    if let Some(id_end) = id_part.find(')') {
                        let id = id_part[..id_end].trim().to_string();
                        let booted = trimmed.contains("Booted");

                        if id.len() == 36 && id.matches('-').count() == 4 {
                            devices.push(Device {
                                id,
                                name,
                                runtime: current_runtime.clone(),
                                booted,
                            });
                        }
                    }
                }
            }
        }
    }

    // Sort devices: booted first, then by name
    devices.sort_by(|a, b| match (a.booted, b.booted) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    Ok(devices)
}

