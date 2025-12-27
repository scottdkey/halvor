//! Docker image build functionality
use anyhow::{Context, Result};
use std::env;
use std::path::PathBuf;
use std::process::Command;

/// Docker image build configuration
pub struct DockerBuildConfig {
    pub dockerfile: PathBuf,
    pub context: PathBuf,
    pub target: Option<String>,
    pub build_args: Vec<(String, String)>,
    pub tags: Vec<String>,
}

impl DockerBuildConfig {
    pub fn new(dockerfile: PathBuf, context: PathBuf) -> Self {
        Self {
            dockerfile,
            context,
            target: None,
            build_args: Vec::new(),
            tags: Vec::new(),
        }
    }

    pub fn with_target(mut self, target: &str) -> Self {
        self.target = Some(target.to_string());
        self
    }

    pub fn with_build_arg(mut self, key: &str, value: &str) -> Self {
        self.build_args.push((key.to_string(), value.to_string()));
        self
    }

    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags.extend(tags);
        self
    }
}

/// Build a Docker image (with multi-platform support)
/// If push is true, builds for multiple platforms and pushes to registry
/// If push is false, builds for current platform only and loads into local Docker
pub fn build_image(config: &DockerBuildConfig) -> Result<()> {
    build_image_with_push(config, false)
}

/// Build a Docker image with push option
pub fn build_image_with_push(config: &DockerBuildConfig, push: bool) -> Result<()> {
    // Check if buildx is available
    let buildx_check = Command::new("docker").args(&["buildx", "version"]).output();

    if buildx_check.is_err() || !buildx_check.unwrap().status.success() {
        println!("⚠️  Docker buildx not available, falling back to regular docker build");
        return build_image_single_platform(config);
    }

    // Determine platforms based on push option
    let platforms = if push {
        // Multi-platform build when pushing
        "linux/amd64,linux/aarch64"
    } else {
        // Single platform for local use
        if cfg!(target_arch = "aarch64") {
            "linux/aarch64"
        } else {
            "linux/amd64"
        }
    };

    let mut docker_args: Vec<String> = vec![
        "buildx".to_string(),
        "build".to_string(),
        "--platform".to_string(),
        platforms.to_string(),
    ];

    // Add build args
    for (key, value) in &config.build_args {
        docker_args.push("--build-arg".to_string());
        docker_args.push(format!("{}={}", key, value));
    }

    // Add tags
    for tag in &config.tags {
        docker_args.push("-t".to_string());
        docker_args.push(tag.clone());
    }

    // Specify Dockerfile
    docker_args.push("-f".to_string());
    docker_args.push(
        config
            .dockerfile
            .to_str()
            .context("Invalid dockerfile path")?
            .to_string(),
    );

    // Add target if specified
    if let Some(ref target) = config.target {
        docker_args.push("--target".to_string());
        docker_args.push(target.clone());
    }

    // Use --push for multi-platform, --load for single platform
    if push {
        docker_args.push("--push".to_string());
        println!(
            "Building multi-platform Docker image (linux/amd64,linux/arm64) and pushing to registry..."
        );
    } else {
        docker_args.push("--load".to_string());
        println!("Building Docker image for platform: {}", platforms);
    }

    // Build context
    docker_args.push(
        config
            .context
            .to_str()
            .context("Invalid context path")?
            .to_string(),
    );

    let status = Command::new("docker")
        .args(&docker_args)
        .status()
        .context("Failed to build Docker image with buildx")?;

    if !status.success() {
        anyhow::bail!("Docker buildx build failed");
    }

    Ok(())
}

/// Build a Docker image (single platform, fallback)
fn build_image_single_platform(config: &DockerBuildConfig) -> Result<()> {
    let mut docker_args: Vec<String> = vec!["build".to_string()];

    // Add build args
    for (key, value) in &config.build_args {
        docker_args.push("--build-arg".to_string());
        docker_args.push(format!("{}={}", key, value));
    }

    // Add tags
    for tag in &config.tags {
        docker_args.push("-t".to_string());
        docker_args.push(tag.clone());
    }

    // Specify Dockerfile
    docker_args.push("-f".to_string());
    docker_args.push(
        config
            .dockerfile
            .to_str()
            .context("Invalid dockerfile path")?
            .to_string(),
    );

    // Add target if specified
    if let Some(ref target) = config.target {
        docker_args.push("--target".to_string());
        docker_args.push(target.clone());
    }

    // Build context
    docker_args.push(
        config
            .context
            .to_str()
            .context("Invalid context path")?
            .to_string(),
    );

    let status = Command::new("docker")
        .args(&docker_args)
        .status()
        .context("Failed to build Docker image")?;

    if !status.success() {
        anyhow::bail!("Docker build failed");
    }

    Ok(())
}

/// Push Docker images to registry
pub fn push_images(tags: &[String]) -> Result<()> {
    for tag in tags {
        println!("Pushing {}...", tag);
        let push_status = Command::new("docker")
            .args(&["push", tag])
            .status()
            .context(format!("Failed to push {}", tag))?;

        if !push_status.success() {
            println!();
            println!("❌ Docker push failed for {}", tag);
            println!();
            println!("This usually means:");
            println!("  1. You're not logged into GitHub Container Registry");
            println!("  2. The package doesn't exist yet (first push requires package creation)");
            println!("  3. You don't have write permissions to the repository");
            println!();
            anyhow::bail!("Push failed for {} - see instructions above", tag);
        }
        println!("✓ Pushed {}", tag);
    }

    Ok(())
}

/// Get GitHub user from environment or git remote
pub fn get_github_user() -> String {
    env::var("GITHUB_USER")
        .or_else(|_| env::var("GITHUB_REPOSITORY_OWNER"))
        .unwrap_or_else(|_| {
            // Try to extract from git remote
            let output = Command::new("git")
                .args(&["remote", "get-url", "origin"])
                .output()
                .ok();
            if let Some(output) = output {
                if let Ok(url) = String::from_utf8(output.stdout) {
                    // Extract username from git@github.com:user/repo.git or https://github.com/user/repo.git
                    if let Some(user) = url
                        .split('/')
                        .nth(if url.contains("github.com:") { 1 } else { 3 })
                        .and_then(|s| s.strip_suffix(".git"))
                    {
                        return user.to_string();
                    }
                }
            }
            "unknown".to_string()
        })
}

/// Get git hash for versioning
pub fn get_git_hash() -> String {
    Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string())
}

/// Check if user is logged into Docker registry
pub fn check_docker_auth() -> Result<()> {
    let login_check = Command::new("docker")
        .args(&["info"])
        .output()
        .context("Failed to check docker info")?;

    if !login_check.status.success() {
        println!("⚠️  Warning: Docker may not be running or accessible");
    }

    Ok(())
}

/// Generate image tags for GitHub Container Registry
pub fn generate_ghcr_tags(
    github_user: &str,
    image_name: &str,
    release: bool,
    git_hash: &str,
) -> Vec<String> {
    let base_image = format!("ghcr.io/{}/{}", github_user, image_name);
    let tag = if release { "development" } else { "dev" };
    let image_tag = format!("{}:{}", base_image, tag);
    let hash_tag = format!("{}:{}", base_image, git_hash);

    let mut tags = vec![image_tag, hash_tag];

    // For release builds, also add experimental tag
    if release {
        let experimental_tag = format!("{}:experimental", base_image);
        tags.push(experimental_tag);
    }

    tags
}

