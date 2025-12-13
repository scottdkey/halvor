// CLI binary build with simple cross-compilation support
use crate::services::build::common::{execute_command_output, get_binary_path};
use crate::services::build::github::push_cli_to_github;
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Command;

/// Platform targets mapping
/// Note: Only macOS targets build reliably on macOS without Docker
/// Linux/Windows targets require 'cross' with Docker running
const PLATFORM_TARGETS: &[(&str, &[&str])] = &[
    // Native macOS builds (no Docker needed)
    ("apple", &["aarch64-apple-darwin", "x86_64-apple-darwin"]),
    // Cross-compilation targets (require Docker via 'cross')
    (
        "linux",
        &[
            "x86_64-unknown-linux-gnu",
            "aarch64-unknown-linux-gnu",
            "x86_64-unknown-linux-musl",
            "aarch64-unknown-linux-musl",
        ],
    ),
    (
        "windows",
        &["x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc"],
    ),
];

/// Build CLI binaries for specified platforms or targets
pub fn build_cli(platforms: Option<&str>, targets: Option<&str>, push: bool) -> Result<()> {
    let platform_targets: HashMap<&str, Vec<&str>> = PLATFORM_TARGETS
        .iter()
        .map(|(k, v)| (*k, v.to_vec()))
        .collect();

    // Get all available target triples for validation
    let all_targets: HashSet<&str> = platform_targets
        .values()
        .flat_map(|v| v.iter())
        .copied()
        .collect();

    // Determine which targets to build
    let targets_to_build: Vec<&str> = if let Some(targets_str) = targets {
        // Build specific targets
        let parsed: Vec<&str> = targets_str
            .split(',')
            .map(|t| t.trim())
            .filter(|t| !t.is_empty())
            .collect();

        if parsed.is_empty() {
            anyhow::bail!("No valid targets specified");
        }

        // Validate that each target is a known target
        for target in &parsed {
            if !all_targets.contains(target) {
                println!(
                    "  ⚠️  Warning: '{}' is not a known target. Attempting to build anyway...",
                    target
                );
                println!("  Known targets are:");
                for platform_target in PLATFORM_TARGETS {
                    println!("    {}: {}", platform_target.0, platform_target.1.join(", "));
                }
            }
        }

        println!(
            "Building CLI binaries for targets: {}",
            parsed.join(", ")
        );

        parsed
    } else if let Some(platforms_str) = platforms {
        // Build specific platforms
        let platforms_to_build: HashSet<&str> = platforms_str
            .split(',')
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .collect();

        if platforms_to_build.is_empty() {
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
        for platform in &platforms_to_build {
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

        println!(
            "Building CLI binaries for platforms: {}",
            platforms_to_build
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        );

        // Collect all targets from selected platforms
        platforms_to_build
            .iter()
            .flat_map(|platform| platform_targets.get(platform).unwrap().iter().copied())
            .collect()
    } else {
        // Build only native macOS targets by default (others require Docker)
        println!("Building CLI binaries for macOS (native targets)");
        println!("💡 Tip: For Linux/Windows builds, ensure Docker is running or use GitHub Actions");
        platform_targets
            .get("apple")
            .map(|v| v.clone())
            .unwrap_or_default()
    };

    // Install all targets first before building
    println!("\n🔧 Ensuring all Rust targets are installed...");
    for target in &targets_to_build {
        if !is_target_installed(target)? {
            println!("  Installing target: {}", target);
            install_target(target)?;
        } else {
            println!("  ✓ Target already installed: {}", target);
        }
    }

    let mut built_binaries: Vec<(String, PathBuf)> = Vec::new();

    // Build for each target
    for target in targets_to_build {
        println!("\n📦 Building target: {}", target);

        // Build for target
        if let Some(binary_path) = build_target(target)? {
            println!("  ✓ Built: {}", binary_path.display());
            built_binaries.push((target.to_string(), binary_path));
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
    // Detect if we're cross-compiling
    let host_target = std::env::var("HOST")
        .or_else(|_| std::env::var("CARGO_BUILD_TARGET"))
        .unwrap_or_else(|_| {
            // Get the current host triple
            #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
            { "aarch64-apple-darwin".to_string() }
            #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
            { "x86_64-apple-darwin".to_string() }
            #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
            { "x86_64-unknown-linux-gnu".to_string() }
            #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
            { "aarch64-unknown-linux-gnu".to_string() }
            #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
            { "x86_64-pc-windows-msvc".to_string() }
            #[cfg(not(any(
                all(target_os = "macos", any(target_arch = "aarch64", target_arch = "x86_64")),
                all(target_os = "linux", any(target_arch = "x86_64", target_arch = "aarch64")),
                all(target_os = "windows", target_arch = "x86_64")
            )))]
            { "unknown".to_string() }
        });

    let is_cross = target != host_target;

    // Cross-compilation detection: Linux/Windows from macOS
    let needs_cross = is_cross && (target.contains("linux") || target.contains("windows"));

    if needs_cross {
        // Cross-compilation is not supported reliably on macOS
        eprintln!("  ⚠️  Skipping {}: Cross-compilation not supported", target);
        eprintln!("     Cross-compiling from macOS to Linux/Windows is unreliable");
        eprintln!("     Use GitHub Actions for production builds (see .github/workflows/)");
        eprintln!("     Each platform builds natively for best results");
        return Ok(None);
    }

    // Native build (macOS only at this point)
    let cargo_cmd = "cargo";
    let cargo_args = vec!["build", "--release", "--bin", "halvor", "--target", target];

    let mut build_cmd = Command::new(cargo_cmd);
    build_cmd.args(&cargo_args);

    // Clear any RUSTFLAGS that might interfere with cross-compilation
    // (cross needs to control the build environment)
    build_cmd.env_remove("RUSTFLAGS");

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
