use anyhow::{Context, Result};
use std::env;
use std::io::Write;
use std::process::{Command, Stdio};

pub fn build_and_push_vpn_image(github_user: &str, image_tag: Option<&str>) -> Result<()> {
    let homelab_dir = crate::config::find_homelab_dir()?;
    let vpn_container_dir = homelab_dir.join("openvpn-container");

    if !vpn_container_dir.exists() {
        anyhow::bail!(
            "VPN container directory not found at {}",
            vpn_container_dir.display()
        );
    }

    // Get git hash for versioning
    let git_hash = Command::new("git")
        .arg("rev-parse")
        .arg("--short")
        .arg("HEAD")
        .current_dir(&homelab_dir)
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
        .unwrap_or_else(|| "unknown".to_string());

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
    let mut build_cmd = Command::new("docker");
    build_cmd.arg("build");
    for tag in &tags_to_push {
        build_cmd.arg("-t").arg(tag);
    }
    build_cmd
        .arg("-f")
        .arg("Dockerfile")
        .arg(".")
        .current_dir(&vpn_container_dir);

    let build_status = build_cmd
        .status()
        .context("Failed to execute docker build")?;

    if !build_status.success() {
        anyhow::bail!("Docker build failed");
    }

    println!("✓ Image built successfully");
    println!();

    // Check if user is logged into GitHub Container Registry
    println!("Checking GitHub Container Registry authentication...");
    let _auth_check = Command::new("docker")
        .arg("info")
        .output()
        .context("Failed to check docker info")?;

    // Try to verify we can access ghcr.io
    let login_test = Command::new("docker")
        .arg("pull")
        .arg(format!("ghcr.io/{}/pia-vpn:latest", github_user))
        .output();

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
        let push_status = Command::new("docker")
            .arg("push")
            .arg(tag)
            .status()
            .context(format!("Failed to execute docker push for {}", tag))?;

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
                "     create it at: https://github.com/{}/pia-vpn",
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

pub fn deploy_vpn(hostname: &str, config: &crate::config::EnvConfig) -> Result<()> {
    let homelab_dir = crate::config::find_homelab_dir()?;

    // Load PIA credentials from local .env
    dotenv::from_path(homelab_dir.join(".env")).context("Failed to load .env file")?;

    let pia_username = env::var("PIA_USERNAME").context("PIA_USERNAME not found in .env file")?;
    let pia_password = env::var("PIA_PASSWORD").context("PIA_PASSWORD not found in .env file")?;

    // Get host configuration
    let host_config = config.hosts.get(hostname).with_context(|| {
        format!(
            "Host '{}' not found in .env\n\nAdd configuration to .env:\n  HOST_{}_IP=\"<ip-address>\"\n  HOST_{}_TAILSCALE=\"<tailscale-hostname>\"",
            hostname,
            hostname.to_uppercase(),
            hostname.to_uppercase()
        )
    })?;

    // Determine which host to connect to (prefer IP, fallback to Tailscale)
    let target_host = if let Some(ip) = &host_config.ip {
        ip.clone()
    } else if let Some(tailscale) = &host_config.tailscale {
        tailscale.clone()
    } else {
        anyhow::bail!("No IP or Tailscale hostname configured for {}", hostname);
    };

    println!("Deploying VPN to {} ({})...", hostname, target_host);
    println!();

    // Read Portainer compose file (for deployment)
    let compose_file = homelab_dir
        .join("compose")
        .join("openvpn-pia-portainer.docker-compose.yml");
    if !compose_file.exists() {
        anyhow::bail!(
            "VPN Portainer compose file not found at {}",
            compose_file.display()
        );
    }

    let compose_content = std::fs::read_to_string(&compose_file)
        .with_context(|| format!("Failed to read compose file: {}", compose_file.display()))?;

    // Don't substitute - let docker-compose read from .env file using --env-file

    // Determine username for SSH
    let default_user = crate::config::get_default_username();
    let host_with_user = format!("{}@{}", default_user, target_host);

    // Test if key-based auth works
    let test_cmd = format!(
        r#"ssh -o ConnectTimeout=1 -o BatchMode=yes -o PreferredAuthentications=publickey -o PasswordAuthentication=no -o StrictHostKeyChecking=no {} 'echo test' >/dev/null 2>&1"#,
        host_with_user
    );

    let test_status = Command::new("sh")
        .arg("-c")
        .arg(&test_cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    let use_key_auth = test_status.is_ok() && test_status.unwrap().success();

    // Create directories on remote system
    let mkdir_command =
        r#"mkdir -p "$HOME/vpn/openvpn" 2>/dev/null || mkdir -p "$(eval echo ~$USER)/vpn/openvpn""#;

    let mut mkdir_cmd = Command::new("ssh");
    if use_key_auth {
        mkdir_cmd.args([
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "PreferredAuthentications=publickey",
            "-o",
            "PasswordAuthentication=no",
            &host_with_user,
            "bash",
            "-c",
            mkdir_command,
        ]);
    } else {
        mkdir_cmd.args([
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "PreferredAuthentications=publickey,keyboard-interactive,password",
            &host_with_user,
            "bash",
            "-c",
            mkdir_command,
        ]);
    }

    mkdir_cmd.stdout(Stdio::null());
    mkdir_cmd.stderr(Stdio::inherit());

    let mkdir_status = mkdir_cmd.status()?;
    if !mkdir_status.success() {
        anyhow::bail!("Failed to create $HOME/vpn/openvpn directory on remote system");
    }

    // Copy OpenVPN config files to remote system
    let openvpn_dir = homelab_dir.join("openvpn");
    let auth_file = openvpn_dir.join("auth.txt");
    let config_file = openvpn_dir.join("ca-montreal.ovpn");

    if !auth_file.exists() {
        anyhow::bail!("OpenVPN auth file not found at {}", auth_file.display());
    }
    if !config_file.exists() {
        anyhow::bail!("OpenVPN config file not found at {}", config_file.display());
    }

    // Copy auth.txt
    let auth_content = std::fs::read_to_string(&auth_file)
        .with_context(|| format!("Failed to read auth file: {}", auth_file.display()))?;
    let auth_setup_cmd =
        r#"cat > "$HOME/vpn/openvpn/auth.txt" || cat > "$(eval echo ~$USER)/vpn/openvpn/auth.txt""#;

    let mut auth_cmd = Command::new("ssh");
    if use_key_auth {
        auth_cmd.args([
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "PreferredAuthentications=publickey",
            "-o",
            "PasswordAuthentication=no",
            &host_with_user,
            "bash",
            "-c",
            auth_setup_cmd,
        ]);
    } else {
        auth_cmd.args([
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "PreferredAuthentications=publickey,keyboard-interactive,password",
            &host_with_user,
            "bash",
            "-c",
            auth_setup_cmd,
        ]);
    }

    auth_cmd.stdin(Stdio::piped());
    auth_cmd.stdout(Stdio::null());
    auth_cmd.stderr(Stdio::inherit());

    let mut auth_child = auth_cmd.spawn()?;
    if let Some(mut stdin) = auth_child.stdin.take() {
        stdin.write_all(auth_content.as_bytes())?;
        stdin.flush()?;
        drop(stdin);
    }

    let auth_status = auth_child.wait()?;
    if !auth_status.success() {
        anyhow::bail!("Failed to copy auth.txt to remote system");
    }

    println!("✓ Copied auth.txt to remote system");

    // Copy ca-montreal.ovpn
    let config_content = std::fs::read_to_string(&config_file)
        .with_context(|| format!("Failed to read config file: {}", config_file.display()))?;
    let config_setup_cmd = r#"cat > "$HOME/vpn/openvpn/ca-montreal.ovpn" || cat > "$(eval echo ~$USER)/vpn/openvpn/ca-montreal.ovpn""#;

    let mut config_cmd = Command::new("ssh");
    if use_key_auth {
        config_cmd.args([
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "PreferredAuthentications=publickey",
            "-o",
            "PasswordAuthentication=no",
            &host_with_user,
            "bash",
            "-c",
            config_setup_cmd,
        ]);
    } else {
        config_cmd.args([
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "PreferredAuthentications=publickey,keyboard-interactive,password",
            &host_with_user,
            "bash",
            "-c",
            config_setup_cmd,
        ]);
    }

    config_cmd.stdin(Stdio::piped());
    config_cmd.stdout(Stdio::null());
    config_cmd.stderr(Stdio::inherit());

    let mut config_child = config_cmd.spawn()?;
    if let Some(mut stdin) = config_child.stdin.take() {
        stdin.write_all(config_content.as_bytes())?;
        stdin.flush()?;
        drop(stdin);
    }

    let config_status = config_child.wait()?;
    if !config_status.success() {
        anyhow::bail!("Failed to copy ca-montreal.ovpn to remote system");
    }

    println!("✓ Copied ca-montreal.ovpn to remote system");

    // Copy compose file to remote system
    let setup_cmd = r#"cat > "$HOME/vpn/docker-compose.yml" || cat > "$(eval echo ~$USER)/vpn/docker-compose.yml""#;

    let mut cmd = Command::new("ssh");
    if use_key_auth {
        cmd.args([
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "PreferredAuthentications=publickey",
            "-o",
            "PasswordAuthentication=no",
            &host_with_user,
            "bash",
            "-c",
            &setup_cmd,
        ]);
    } else {
        cmd.args([
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "PreferredAuthentications=publickey,keyboard-interactive,password",
            &host_with_user,
            "bash",
            "-c",
            &setup_cmd,
        ]);
    }

    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::inherit());

    let mut child = cmd.spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(compose_content.as_bytes())?;
        stdin.flush()?;
        drop(stdin);
    }

    let status = child.wait()?;
    if !status.success() {
        anyhow::bail!("Failed to copy VPN compose file to remote system");
    }

    println!("✓ Copied VPN compose file to remote system");

    // Create .env file on remote system with PIA credentials
    let env_content = format!(
        "PIA_USERNAME={}\nPIA_PASSWORD={}\n",
        pia_username, pia_password
    );
    let env_setup_cmd = r#"cat > "$HOME/vpn/.env" || cat > "$(eval echo ~$USER)/vpn/.env""#;

    let mut env_cmd = Command::new("ssh");
    if use_key_auth {
        env_cmd.args([
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "PreferredAuthentications=publickey",
            "-o",
            "PasswordAuthentication=no",
            &host_with_user,
            "bash",
            "-c",
            &env_setup_cmd,
        ]);
    } else {
        env_cmd.args([
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "PreferredAuthentications=publickey,keyboard-interactive,password",
            &host_with_user,
            "bash",
            "-c",
            &env_setup_cmd,
        ]);
    }

    env_cmd.stdin(Stdio::piped());
    env_cmd.stdout(Stdio::null());
    env_cmd.stderr(Stdio::inherit());

    let mut env_child = env_cmd.spawn()?;
    if let Some(mut stdin) = env_child.stdin.take() {
        stdin.write_all(env_content.as_bytes())?;
        stdin.flush()?;
        drop(stdin);
    }

    let env_status = env_child.wait()?;
    if !env_status.success() {
        anyhow::bail!("Failed to create .env file on remote system");
    }

    println!("✓ Created .env file on remote system");

    println!();
    println!(
        "✓ VPN configuration files copied to {} ({})",
        hostname, target_host
    );
    println!("  Files copied:");
    println!("    - ~/vpn/docker-compose.yml (Portainer compose file)");
    println!("    - ~/vpn/.env (PIA credentials)");
    println!("    - ~/vpn/openvpn/auth.txt (OpenVPN authentication)");
    println!("    - ~/vpn/openvpn/ca-montreal.ovpn (OpenVPN configuration)");
    println!();
    println!("  You can now deploy the VPN manually using Portainer or docker-compose.");

    Ok(())
}
