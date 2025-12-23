use crate::config::EnvConfig;
use crate::utils::exec::{CommandExecutor, Executor};
use anyhow::{Context, Result};

pub fn build_and_push_vpn_image(
    hostname: &str,
    github_user: &str,
    image_tag: Option<&str>,
    config: &EnvConfig,
) -> Result<()> {
    // Create executor - it automatically determines if execution should be local or remote
    let exec = Executor::new(hostname, config)?;
    let target_host = exec.target_host(hostname, config)?;
    let is_local = exec.is_local();

    if is_local {
        println!("Building VPN container image locally on {}...", hostname);
    } else {
        println!(
            "Building VPN container image on {} ({})...",
            hostname, target_host
        );
    }
    println!();

    // For local builds, use local filesystem
    // For remote builds, we'd need to handle file paths differently (not implemented yet)
    let halvor_dir = if is_local {
        crate::config::find_halvor_dir()?
    } else {
        anyhow::bail!(
            "Remote builds require the VPN container directory to be available on the remote host. This is not yet implemented."
        );
    };
    let vpn_container_dir = halvor_dir.join("openvpn-container");

    if is_local && !vpn_container_dir.exists() {
        anyhow::bail!(
            "VPN container directory not found at {}",
            vpn_container_dir.display()
        );
    }

    // Get git hash for versioning (only works locally where git repo exists)
    let git_hash = if is_local {
        exec.execute_simple("git", &["rev-parse", "--short", "HEAD"])
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
    } else {
        "unknown".to_string()
    };

    let base_image = format!("ghcr.io/{}/pia-vpn", github_user);
    let latest_tag = format!("{}:latest", base_image);
    let hash_tag = format!("{}:{}", base_image, git_hash);

    // Use custom tag if provided, otherwise use both latest and hash
    let tags_to_push = if let Some(custom_tag) = image_tag {
        vec![format!("{}:{}", base_image, custom_tag)]
    } else {
        vec![latest_tag.clone(), hash_tag.clone()]
    };

    println!("Building VPN container image...");
    println!("  Tags: {}", tags_to_push.join(", "));
    println!();

    // Build the image with all tags
    let mut build_args = vec!["build"];
    for tag in &tags_to_push {
        build_args.push("-t");
        build_args.push(tag);
    }
    build_args.extend(&["-f", "Dockerfile", "."]);

    // For local builds, we need to change directory, so use shell command
    // For remote builds, we'd need to copy files first (not implemented yet)
    if !is_local {
        anyhow::bail!("Remote Docker builds are not yet supported. Please build locally.");
    }

    // Build using executor (for local, this will use local execution)
    let build_cmd = format!(
        "cd {} && docker {}",
        vpn_container_dir.display(),
        build_args.join(" ")
    );
    let build_output = exec
        .execute_shell(&build_cmd)
        .context("Failed to execute docker build")?;

    if !build_output.status.success() {
        anyhow::bail!("Docker build failed");
    }

    println!("✓ Image built successfully");
    println!();

    // Check if user is logged into GitHub Container Registry
    println!("Checking GitHub Container Registry authentication...");
    let _auth_check = exec
        .execute_simple("docker", &["info"])
        .context("Failed to check docker info")?;

    // Try to verify we can access ghcr.io
    let login_test = exec.execute_simple(
        "docker",
        &["pull", &format!("ghcr.io/{}/pia-vpn:latest", github_user)],
    );

    if let Ok(output) = login_test {
        if !output.status.success() {
            println!("⚠ Warning: Not authenticated or package doesn't exist yet");
            println!("  You may need to login first:");
            println!(
                "  echo $GITHUB_TOKEN | docker login ghcr.io -u {} --password-stdin",
                github_user
            );
            println!();
        }
    }

    println!("Pushing images to GitHub Container Registry...");
    println!();

    // Push all tags
    for tag in &tags_to_push {
        println!("Pushing {}...", tag);
        let push_output = exec
            .execute_simple("docker", &["push", tag])
            .with_context(|| format!("Failed to execute docker push for {}", tag))?;
        let push_status = push_output.status;

        if !push_status.success() {
            println!();
            println!("❌ Docker push failed for {}", tag);
            println!();
            println!("This usually means:");
            println!("  1. You're not logged into GitHub Container Registry");
            println!("  2. The package doesn't exist yet (first push requires package creation)");
            println!("  3. You don't have write permissions to the repository");
            println!();
            println!("To fix:");
            println!(
                "  1. Create a GitHub Personal Access Token (PAT) with 'write:packages' permission"
            );
            println!("  2. Login to GitHub Container Registry:");
            println!(
                "     echo $GITHUB_TOKEN | docker login ghcr.io -u {} --password-stdin",
                github_user
            );
            println!();
            println!("  3. If this is the first push, make sure the repository exists or");
            println!(
                "     create it at: https://github.com/users/{}/packages/container/vpn",
                github_user
            );
            println!();
            anyhow::bail!("Push failed - see instructions above");
        }
        println!("✓ Pushed {}", tag);
    }

    println!();
    println!("✓ All images pushed successfully");
    println!();
    println!("To use this image, set in your .env file:");
    println!("  VPN_IMAGE={}", latest_tag);
    println!();
    println!("Or update compose/openvpn-pia.docker-compose.yml to use:");
    println!("  image: {}", latest_tag);
    if !git_hash.is_empty() && git_hash != "unknown" {
        println!("  # Or use specific version: image: {}", hash_tag);
    }

    Ok(())
}
