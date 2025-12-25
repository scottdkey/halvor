// Web application build (Rust server + Svelte frontend)
use crate::services::build::common::execute_command;
use crate::services::docker::build::{
    DockerBuildConfig, build_image_with_push, check_docker_auth, generate_ghcr_tags, get_git_hash,
    get_github_user,
};
use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;

/// Build web application (Rust server + Svelte frontend) for bare metal
pub fn build_web(release: bool) -> Result<()> {
    println!("Building web application...");

    // Step 1: Build the Rust server binary
    println!("Building Rust web server...");
    let mut cargo_args = vec!["build", "--bin", "halvor"];

    if release {
        cargo_args.push("--release");
        println!("Building in release mode...");
    }

    let mut cargo_cmd = Command::new("cargo");
    cargo_cmd.args(&cargo_args);
    execute_command(cargo_cmd, "Rust server build failed")?;

    // Step 2: Build the Svelte frontend
    println!("Building Svelte frontend...");
    let web_dir = PathBuf::from("projects/web");

    // Check if node_modules exists, install if needed
    if !web_dir.join("node_modules").exists() {
        println!("Installing npm dependencies...");
        let mut npm_cmd = Command::new("npm");
        npm_cmd.arg("install").current_dir(&web_dir);
        execute_command(npm_cmd, "npm install failed")?;
    }

    // Build Svelte app
    let mut npm_args = vec!["run", "build"];
    if release {
        npm_args.push("--");
        npm_args.push("--mode");
        npm_args.push("production");
    }

    let mut npm_cmd = Command::new("npm");
    npm_cmd.args(&npm_args).current_dir(&web_dir);
    execute_command(npm_cmd, "Svelte build failed")?;

    println!("✓ Web build complete!");
    println!(
        "  - Rust server: target/{}/halvor",
        if release { "release" } else { "debug" }
    );
    println!("  - Svelte app: projects/web/build/");
    println!("\nTo run the server:");
    println!("  halvor dev web --bare-metal");
    if release {
        println!("  or: cargo run --release --bin halvor -- dev web --bare-metal");
    } else {
        println!("  or: cargo run --bin halvor -- dev web --bare-metal");
    }

    Ok(())
}

/// Build web application as Docker container
pub fn build_web_docker(release: bool, push: bool) -> Result<()> {
    println!("Building Docker container for web application...");

    // Get GitHub user and git hash
    let github_user = get_github_user();
    if github_user == "unknown" {
        println!(
            "⚠️  Warning: Could not determine GitHub user. Set GITHUB_USER environment variable."
        );
        println!("   Using 'unknown' as image name prefix.");
    }

    let git_hash = get_git_hash();

    // Generate image tags
    let tags = generate_ghcr_tags(&github_user, "halvor-web", release, &git_hash); // Keep image name as halvor-web for registry

    println!("Building Docker image...");
    for tag in &tags {
        println!("  Tag: {}", tag);
    }
    println!("  Release mode: {}", release);
    println!();

    // Build Docker image using docker_build module
    let dockerfile = PathBuf::from("projects/web/Dockerfile");
    let context = PathBuf::from(".");

    let build_config = DockerBuildConfig::new(dockerfile, context)
        .with_target("production")
        .with_build_arg("BUILD_TYPE", if release { "release" } else { "debug" })
        .with_tags(tags.clone());

    if push {
        println!(
            "Building multi-platform image (linux/amd64,linux/arm64) and pushing to registry..."
        );
        check_docker_auth()?;
    }

    build_image_with_push(&build_config, push)?;

    println!("✓ Docker image built successfully");
    for tag in &tags {
        println!("  - {}", tag);
    }
    println!();

    if push {
        println!("✓ Multi-platform images pushed successfully");
        println!();
        println!("To use this image:");
        println!("  docker run -p 13000:13000 {}", tags[0]);
    } else {
        println!("To build and push multi-platform image:");
        println!("  halvor build web --release --push");
    }

    Ok(())
}

/// Run web application in production mode using Docker Compose
pub fn run_web_prod() -> Result<()> {
    println!("Starting web app in production mode (Docker)...");
    let web_dir = PathBuf::from("projects/web");
    let mut docker_cmd = Command::new("docker-compose");
    docker_cmd.args(["up", "--build"]).current_dir(&web_dir);

    execute_command(docker_cmd, "Failed to run web production container")?;

    Ok(())
}
