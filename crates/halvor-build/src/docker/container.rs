//! Docker container build functionality
use anyhow::{Context, Result};
use std::path::PathBuf;

/// Container definitions for buildable containers
struct ContainerDef {
    name: &'static str,
    build_dir: &'static str,
    image_name: &'static str,
}

/// Registry of buildable containers
const CONTAINERS: &[ContainerDef] = &[ContainerDef {
    name: "pia-vpn",
    build_dir: "crates/halvor-openvpn",
    image_name: "ghcr.io/scottdkey/pia-vpn",
}];

/// Build container using its own build script
fn build_container_with_script(
    build_script: &std::path::Path,
    build_dir: &std::path::Path,
    push: bool,
    release: bool,
    no_cache: bool,
) -> Result<()> {
    println!("Using container's build script: {}", build_script.display());
    println!();

    let mut cmd = std::process::Command::new("bash");
    cmd.arg(build_script);

    if push {
        cmd.arg("--push");
    }

    if release {
        cmd.arg("--release");
    }

    if no_cache {
        cmd.arg("--no-cache");
    }

    // Set GITHUB_USER from environment or use default
    if let Ok(user) = std::env::var("GITHUB_USER") {
        cmd.arg("--github-user").arg(&user);
    }

    cmd.current_dir(build_dir);
    cmd.stdout(std::process::Stdio::inherit());
    cmd.stderr(std::process::Stdio::inherit());

    let status = cmd.status().context("Failed to execute build script")?;

    if !status.success() {
        anyhow::bail!("Container build script failed");
    }

    Ok(())
}

/// Build a Docker container
pub fn build_container(name: &str, no_cache: bool, push: bool, release: bool) -> Result<()> {
    // Find container definition
    let container = CONTAINERS
        .iter()
        .find(|c| c.name == name)
        .ok_or_else(|| anyhow::anyhow!("Unknown container: {}", name))?;

    // Find the build directory
    let build_path = find_container_build_path(container.build_dir)?;

    // Check if container has its own build script
    let build_script = build_path.join("build.sh");
    if build_script.exists() {
        return build_container_with_script(&build_script, &build_path, push, release, no_cache);
    }

    println!(
        "Building {} from {}...",
        container.name,
        build_path.display()
    );

    // Determine the tag
    let tag = if release { "latest" } else { "experimental" };
    let full_image = format!("{}:{}", container.image_name, tag);

    // Use buildx for multi-platform builds
    let platforms = "linux/amd64,linux/arm64";

    println!("Building multi-platform image for: {}", platforms);
    println!("  Image: {}", full_image);

    // Check if buildx is available
    let buildx_check = std::process::Command::new("docker")
        .args(&["buildx", "version"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    if buildx_check.is_err() || !buildx_check.unwrap().success() {
        anyhow::bail!(
            "Docker buildx is required for multi-platform builds. Install it with: docker buildx install"
        );
    }

    // Create buildx builder if it doesn't exist
    let builder_name = "halvor-builder";
    let builder_check = std::process::Command::new("docker")
        .args(&["buildx", "inspect", builder_name])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    if builder_check.is_err() || !builder_check.unwrap().success() {
        println!("  Creating buildx builder: {}", builder_name);
        std::process::Command::new("docker")
            .args(&["buildx", "create", "--name", builder_name, "--use"])
            .status()
            .context("Failed to create buildx builder")?;
    } else {
        // Use the existing builder
        std::process::Command::new("docker")
            .args(&["buildx", "use", builder_name])
            .status()
            .context("Failed to use buildx builder")?;
    }

    // Build with buildx
    let build_path_str = build_path.to_string_lossy();
    let mut build_args = vec![
        "buildx",
        "build",
        "--platform",
        platforms,
        "-t",
        &full_image,
    ];

    if no_cache {
        build_args.push("--no-cache");
    }

    if push {
        build_args.push("--push");
    } else {
        // For multi-platform builds, we need to push or use a single platform
        // Since user wants universal build, we'll build for both but only load the native one
        // Actually, --load doesn't work with multi-platform, so we'll build for current platform only when not pushing
        println!(
            "  Note: Multi-platform builds require --push. Building for current platform only for local testing."
        );
        // Remove platform arg and use regular build for local
        build_args = vec!["build", "-t", &full_image];
        if no_cache {
            build_args.push("--no-cache");
        }
        build_args.push(&build_path_str);

        println!("  Running: docker {}", build_args.join(" "));

        let output = std::process::Command::new("docker")
            .args(&build_args)
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()
            .with_context(|| "Failed to run docker build")?;

        if !output.success() {
            anyhow::bail!("Docker build failed");
        }

        println!("✓ Built {} (local platform only)", full_image);
        return Ok(());
    }

    build_args.push(&build_path_str);

    println!("  Running: docker {}", build_args.join(" "));

    let output = std::process::Command::new("docker")
        .args(&build_args)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .with_context(|| "Failed to run docker buildx build")?;

    if !output.success() {
        anyhow::bail!("Docker buildx build failed");
    }

    println!("✓ Built {}", full_image);

    if push {
        println!("✓ Pushed {} to registry", full_image);
    }

    Ok(())
}

/// Find the container build directory path
fn find_container_build_path(build_dir: &str) -> Result<PathBuf> {
    use std::path::Path;

    // Try relative to current directory first
    let relative = Path::new(build_dir);
    if relative.exists() && relative.join("Dockerfile").exists() {
        return Ok(relative.to_path_buf());
    }

    // Try relative to executable (for installed binary)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let dev_path = exe_dir
                .parent()
                .and_then(|p| p.parent())
                .map(|p| p.join(build_dir));

            if let Some(path) = dev_path {
                if path.exists() && path.join("Dockerfile").exists() {
                    return Ok(path);
                }
            }
        }
    }

    // Try from CARGO_MANIFEST_DIR (development)
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let path = Path::new(&manifest_dir).join(build_dir);
        if path.exists() && path.join("Dockerfile").exists() {
            return Ok(path);
        }
    }

    anyhow::bail!(
        "Could not find build directory '{}' with Dockerfile. Make sure {}/Dockerfile exists.",
        build_dir,
        build_dir
    )
}

