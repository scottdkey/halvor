// CLI binary build with simple cross-compilation support
use crate::services::build::common::{execute_command_output, get_binary_path};
use crate::services::build::github::push_cli_to_github;
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Command;

/// Platform targets mapping
/// All platforms support cross-compilation via 'cross' tool (requires Docker)
const PLATFORM_TARGETS: &[(&str, &[&str])] = &[
    // Apple/macOS targets
    ("apple", &["aarch64-apple-darwin", "x86_64-apple-darwin"]),
    // Linux targets (cross-compile from any platform using 'cross')
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

/// Build CLI binary for current platform and push to experimental release
pub fn build_and_push_experimental() -> Result<()> {
    use std::env::consts::{ARCH, OS};
    
    // Detect current platform
    let current_target = match (OS, ARCH) {
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        _ => anyhow::bail!("Unsupported platform: {} {}", OS, ARCH),
    };
    
    println!("Building halvor for current platform: {}", current_target);
    println!("This will be pushed to the 'experimental' GitHub release\n");
    
    // Build for current target
    let binary_path = build_target(current_target)?
        .context(format!("Failed to build for target: {}", current_target))?;
    
    println!("✓ Build successful: {}", binary_path.display());
    
    // Push to experimental release
    println!("\n📤 Pushing to GitHub 'experimental' release...");
    push_cli_to_github(&[(current_target.to_string(), binary_path)], Some("experimental"))?;
    
    println!("\n✓ Successfully pushed to experimental release!");
    println!("  Download URL: https://github.com/scottdkey/halvor/releases/tag/experimental");
    
    Ok(())
}

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
                    println!(
                        "    {}: {}",
                        platform_target.0,
                        platform_target.1.join(", ")
                    );
                }
            }
        }

        println!("Building CLI binaries for targets: {}", parsed.join(", "));

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
        // Build for current platform by default
        use std::env::consts::OS;
        let current_platform = match OS {
            "macos" => "apple",
            "linux" => "linux",
            "windows" => "windows",
            _ => {
                anyhow::bail!(
                    "Unsupported platform: {}. Please specify --platforms explicitly",
                    OS
                );
            }
        };

        println!(
            "Building CLI binaries for {} (native targets)",
            current_platform
        );
        println!(
            "💡 Note: Cross-platform builds are handled via GitHub Actions workflows.\n\
             For local development, only native targets are built."
        );

        platform_targets
            .get(current_platform)
            .map(|v| v.clone())
            .unwrap_or_default()
    };

    // Install all targets first before building
    println!("\n🔧 Ensuring all Rust targets are installed...");
    for target in &targets_to_build {
        if !is_target_installed(target)? {
            println!("  Installing target: {}", target);
            install_target(target)?;
            // Verify installation succeeded
            if !is_target_installed(target)? {
                anyhow::bail!(
                    "Failed to install target {} - installation reported success but target is not available",
                    target
                );
            }
            println!("  ✓ Successfully installed target: {}", target);
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
        push_cli_to_github(&built_binaries, None)?;
    }

    Ok(())
}

/// Check if a Rust target is installed
fn is_target_installed(target: &str) -> Result<bool> {
    let mut cmd = Command::new("rustup");
    cmd.args(["target", "list", "--installed"]);
    let output = execute_command_output(cmd, "Failed to check installed targets")?;

    let installed_targets = String::from_utf8_lossy(&output.stdout);
    // Check for exact match (target should be on its own line, trimmed)
    let is_installed = installed_targets.lines().any(|line| line.trim() == target);
    Ok(is_installed)
}

/// Install a Rust target
fn install_target(target: &str) -> Result<()> {
    // rustup target add doesn't support --force-non-host flag
    // The target should install fine if the toolchain supports it
    let status = Command::new("rustup")
        .args(["target", "add", target])
        .status()
        .context(format!("Failed to install target: {}", target))?;

    if !status.success() {
        // Get the error output to provide better diagnostics
        let output = Command::new("rustup")
            .args(["target", "add", target])
            .output()
            .context(format!("Failed to run rustup target add for {}", target))?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "Failed to install target {}: {}\n\
            Note: For cross-compilation, 'cross' uses Docker containers with pre-installed toolchains.\n\
            Ensure Docker is running and the target is available for your Rust toolchain.",
            target,
            stderr
        );
    }

    // Verify the target was actually installed
    if !is_target_installed(target)? {
        anyhow::bail!(
            "Target {} was not installed successfully even though rustup reported success",
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
            // Get the current host triple at RUNTIME (not compile-time)
            // This ensures correct detection when SSH'd into a different platform
            use std::env::consts::{ARCH, OS};
            match (OS, ARCH) {
                ("macos", "aarch64") => "aarch64-apple-darwin".to_string(),
                ("macos", "x86_64") => "x86_64-apple-darwin".to_string(),
                ("linux", "x86_64") => "x86_64-unknown-linux-gnu".to_string(),
                ("linux", "aarch64") => "aarch64-unknown-linux-gnu".to_string(),
                ("windows", "x86_64") => "x86_64-pc-windows-msvc".to_string(),
                _ => "unknown".to_string(),
            }
        });

    let is_cross = target != host_target;

    // Determine if we need to use 'cross' for cross-compilation
    // Use 'cross' for cross-OS compilation (e.g., macOS -> Linux/Windows)
    // Use 'cargo' for native builds or same-OS cross-arch builds
    use std::env::consts::OS;
    let host_os = OS;
    let target_os = if target.contains("linux") {
        "linux"
    } else if target.contains("windows") {
        "windows"
    } else if target.contains("darwin") || target.contains("apple") {
        "macos"
    } else {
        "unknown"
    };

    let needs_cross = is_cross && host_os != target_os;

    // Cross-compilation is handled via GitHub Actions, not locally
    // Only build native targets locally
    if needs_cross {
        println!(
            "  ⚠️  Skipping cross-compilation for target: {}\n\
             Cross-platform builds are handled via GitHub Actions workflows.\n\
             See .github/workflows/ for build configurations.",
            target
        );
        return Ok(None);
    }

    // Use cargo for native builds only
    let cargo_cmd = "cargo";
    let cargo_args = vec!["build", "--release", "--bin", "halvor", "--target", target];

    let mut build_cmd = Command::new(cargo_cmd);
    build_cmd.args(&cargo_args);

    // Clear any RUSTFLAGS that might interfere with compilation
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
