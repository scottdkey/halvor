// CLI binary build with cross-compilation support
use crate::services::build::common::{execute_command_output, get_binary_path};
use crate::services::build::github::push_cli_to_github;
use crate::services::build::zig::setup_zig_cross_compilation;
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Command;

/// Platform targets mapping
const PLATFORM_TARGETS: &[(&str, &[&str])] = &[
    ("apple", &["aarch64-apple-darwin", "x86_64-apple-darwin"]),
    (
        "windows",
        &["x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc"],
    ),
    (
        "linux",
        &[
            "x86_64-unknown-linux-gnu",
            "aarch64-unknown-linux-gnu",
            "x86_64-unknown-linux-musl",
            "aarch64-unknown-linux-musl",
        ],
    ),
];

/// Build CLI binaries for specified platforms
pub fn build_cli(platforms: Option<&str>, push: bool) -> Result<()> {
    let platform_targets: HashMap<&str, Vec<&str>> = PLATFORM_TARGETS
        .iter()
        .map(|(k, v)| (*k, v.to_vec()))
        .collect();

    // Determine which platforms to build
    let platforms_to_build: HashSet<&str> = if let Some(platforms_str) = platforms {
        let parsed: HashSet<&str> = platforms_str
            .split(',')
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .collect();

        if parsed.is_empty() {
            anyhow::bail!(
                "No valid platforms specified. Valid platforms are: {}",
                platform_targets
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        // Validate platforms
        for platform in &parsed {
            if !platform_targets.contains_key(platform) {
                anyhow::bail!(
                    "Unknown platform: '{}'. Valid platforms are: {}\n\nHint: Use comma-separated values like: --platforms apple,windows,linux",
                    platform,
                    platform_targets
                        .keys()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
        }

        parsed
    } else {
        // Build all platforms
        platform_targets.keys().copied().collect()
    };

    println!(
        "Building CLI binaries for platforms: {}",
        platforms_to_build
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    );

    let mut built_binaries: Vec<(String, PathBuf)> = Vec::new();

    // Build for each platform
    for platform in &platforms_to_build {
        let targets = platform_targets.get(platform).unwrap();
        println!("\n📦 Building for {} platform...", platform);

        for target in targets {
            println!("  Building target: {}", target);

            // Ensure target is installed
            if !is_target_installed(target)? {
                println!("  Installing target: {}", target);
                install_target(target)?;
            }

            // Build for target
            if let Some(binary_path) = build_target(target)? {
                println!("  ✓ Built: {}", binary_path.display());
                built_binaries.push((target.to_string(), binary_path));
            }
        }
    }

    if built_binaries.is_empty() {
        anyhow::bail!("No binaries were built successfully");
    }

    println!("\n✓ Built {} binary(ies)", built_binaries.len());
    for (target, path) in &built_binaries {
        println!("  - {}: {}", target, path.display());
    }

    // Push to GitHub releases if requested
    if push {
        println!("\n📤 Pushing to GitHub releases...");
        push_cli_to_github(&built_binaries)?;
    }

    Ok(())
}

/// Check if a Rust target is installed
fn is_target_installed(target: &str) -> Result<bool> {
    let mut cmd = Command::new("rustup");
    cmd.args(["target", "list", "--installed"]);
    let output = execute_command_output(cmd, "Failed to check installed targets")?;

    let installed_targets = String::from_utf8_lossy(&output.stdout);
    Ok(installed_targets.contains(target))
}

/// Install a Rust target
fn install_target(target: &str) -> Result<()> {
    let status = Command::new("rustup")
        .args(["target", "add", target])
        .status()
        .context(format!("Failed to install target: {}", target))?;

    if !status.success() {
        eprintln!(
            "  ⚠️  Warning: Failed to install target {}, skipping",
            target
        );
    }
    Ok(())
}

/// Build for a specific target
pub fn build_target(target: &str) -> Result<Option<PathBuf>> {
    let mut build_cmd = Command::new("cargo");
    build_cmd.args(["build", "--release", "--bin", "halvor", "--target", target]);

    // Configure Zig for cross-compilation on macOS for Linux and Windows targets
    #[cfg(target_os = "macos")]
    if target.contains("linux") || target.contains("windows") {
        setup_zig_cross_compilation(&mut build_cmd, target)?;
    }

    // Build for target
    let build_result = build_cmd.output();

    match build_result {
        Ok(output) if output.status.success() => {
            // Build succeeded, find the binary
            let binary_path = get_binary_path(target, true);
            if binary_path.exists() {
                Ok(Some(binary_path))
            } else {
                eprintln!(
                    "  ⚠️  Warning: Binary not found at: {}",
                    binary_path.display()
                );
                Ok(None)
            }
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("  ❌ Build failed for target: {}", target);
            if !stderr.is_empty() {
                // Show last few lines of error
                let error_lines: Vec<&str> = stderr.lines().rev().take(5).collect();
                for line in error_lines.iter().rev() {
                    eprintln!("     {}", line);
                }
            }
            Ok(None)
        }
        Err(e) => {
            eprintln!("  ❌ Failed to execute build for target: {}: {}", target, e);
            Ok(None)
        }
    }
}
