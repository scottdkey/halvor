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

    let base_image = format!("ghcr.io/{}/vpn", github_user);
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
        .arg(format!("ghcr.io/{}/vpn:latest", github_user))
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

    // Read compose file - use local build version for now (avoids registry auth issues)
    // User can switch to portainer version after making image public
    let compose_file = homelab_dir
        .join("compose")
        .join("openvpn-pia.docker-compose.yml");
    if !compose_file.exists() {
        anyhow::bail!("VPN compose file not found at {}", compose_file.display());
    }

    let compose_content = std::fs::read_to_string(&compose_file)
        .with_context(|| format!("Failed to read compose file: {}", compose_file.display()))?;

    // Don't substitute - let docker-compose read from .env file using --env-file

    // Determine username for SSH and VPN config path
    let default_user = crate::config::get_default_username();
    // Allow VPN_USER to override the username for config path (useful for Portainer)
    let vpn_user = env::var("VPN_USER").unwrap_or_else(|_| default_user.clone());
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

    // Check if files already exist - if so, skip deployment
    // Check for both .ovpn and .opvn (typo) variants
    // Use /home/$USER/config/vpn (USER can be set via VPN_USER env var)
    let vpn_config_dir = format!("/home/{}/config/vpn", vpn_user);
    let check_cmd = format!(
        r#"test -f "{}/auth.txt" && (test -f "{}/ca-montreal.ovpn" || test -f "{}/ca-montreal.opvn") && echo 'exists' || echo 'missing'"#,
        vpn_config_dir, vpn_config_dir, vpn_config_dir
    );
    let mut check_cmd_exec = Command::new("ssh");
    if use_key_auth {
        check_cmd_exec.args([
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "PreferredAuthentications=publickey",
            "-o",
            "PasswordAuthentication=no",
            &host_with_user,
            "bash",
            "-c",
            &check_cmd,
        ]);
    } else {
        check_cmd_exec.args([
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "PreferredAuthentications=publickey,keyboard-interactive,password",
            &host_with_user,
            "bash",
            "-c",
            &check_cmd,
        ]);
    }
    check_cmd_exec.stdout(Stdio::piped());
    check_cmd_exec.stderr(Stdio::null());
    let check_output = check_cmd_exec.output()?;
    let output_str = String::from_utf8_lossy(&check_output.stdout);
    let files_exist = output_str.trim() == "exists";

    // Debug: print what we got
    if !files_exist {
        eprintln!(
            "Debug: Check command output: '{}' (status: {})",
            output_str.trim(),
            check_output.status
        );
    }

    if files_exist {
        println!("✓ VPN configuration files already exist on remote system");
        println!("  Skipping file copy (files are already in place)");
    } else {
        println!("VPN configuration files not found, attempting to copy...");

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

        // Copy files using scp, then move to $HOME/config/vpn
        // First copy to temp location in home directory, then move to final location
        let mut scp_auth = Command::new("scp");
        if use_key_auth {
            scp_auth.args([
                "-o",
                "StrictHostKeyChecking=no",
                "-o",
                "PreferredAuthentications=publickey",
                "-o",
                "PasswordAuthentication=no",
                auth_file.to_str().unwrap(),
                &format!("{}:~/auth.txt.tmp", host_with_user),
            ]);
        } else {
            scp_auth.args([
                "-o",
                "StrictHostKeyChecking=no",
                "-o",
                "PreferredAuthentications=publickey,keyboard-interactive,password",
                auth_file.to_str().unwrap(),
                &format!("{}:~/auth.txt.tmp", host_with_user),
            ]);
        }
        scp_auth.stdout(Stdio::null());
        scp_auth.stderr(Stdio::inherit());
        let scp_auth_status = scp_auth.status()?;
        if !scp_auth_status.success() {
            anyhow::bail!("Failed to copy auth.txt to remote system");
        }

        // Move and set permissions (no sudo needed in user's home directory)
        let move_auth_cmd = format!(
            r#"mkdir -p "/home/{}/config/vpn" && mv ~/auth.txt.tmp "/home/{}/config/vpn/auth.txt" && chmod 600 "/home/{}/config/vpn/auth.txt""#,
            vpn_user, vpn_user, vpn_user
        );
        let mut move_auth = Command::new("ssh");
        if use_key_auth {
            move_auth.args([
                "-o",
                "StrictHostKeyChecking=no",
                "-o",
                "PreferredAuthentications=publickey",
                "-o",
                "PasswordAuthentication=no",
                &host_with_user,
                "bash",
                "-c",
                &move_auth_cmd,
            ]);
        } else {
            move_auth.args([
                "-o",
                "StrictHostKeyChecking=no",
                "-o",
                "PreferredAuthentications=publickey,keyboard-interactive,password",
                &host_with_user,
                "bash",
                "-c",
                &move_auth_cmd,
            ]);
        }
        move_auth.stdout(Stdio::null());
        move_auth.stderr(Stdio::inherit());
        let move_auth_status = move_auth.status()?;
        if !move_auth_status.success() {
            anyhow::bail!("Failed to move auth.txt to $HOME/config/vpn");
        }
        println!("✓ Copied auth.txt to remote system");

        // Copy config file
        let mut scp_config = Command::new("scp");
        if use_key_auth {
            scp_config.args([
                "-o",
                "StrictHostKeyChecking=no",
                "-o",
                "PreferredAuthentications=publickey",
                "-o",
                "PasswordAuthentication=no",
                config_file.to_str().unwrap(),
                &format!("{}:~/ca-montreal.ovpn.tmp", host_with_user),
            ]);
        } else {
            scp_config.args([
                "-o",
                "StrictHostKeyChecking=no",
                "-o",
                "PreferredAuthentications=publickey,keyboard-interactive,password",
                config_file.to_str().unwrap(),
                &format!("{}:~/ca-montreal.ovpn.tmp", host_with_user),
            ]);
        }
        scp_config.stdout(Stdio::null());
        scp_config.stderr(Stdio::inherit());
        let scp_config_status = scp_config.status()?;
        if !scp_config_status.success() {
            anyhow::bail!("Failed to copy ca-montreal.ovpn to remote system");
        }

        // Move and set permissions (no sudo needed in user's home directory)
        let move_config_cmd = format!(
            r#"mkdir -p "/home/{}/config/vpn" && mv ~/ca-montreal.ovpn.tmp "/home/{}/config/vpn/ca-montreal.ovpn" && chmod 644 "/home/{}/config/vpn/ca-montreal.ovpn""#,
            vpn_user, vpn_user, vpn_user
        );
        let mut move_config = Command::new("ssh");
        if use_key_auth {
            move_config.args([
                "-o",
                "StrictHostKeyChecking=no",
                "-o",
                "PreferredAuthentications=publickey",
                "-o",
                "PasswordAuthentication=no",
                &host_with_user,
                "bash",
                "-c",
                &move_config_cmd,
            ]);
        } else {
            move_config.args([
                "-o",
                "StrictHostKeyChecking=no",
                "-o",
                "PreferredAuthentications=publickey,keyboard-interactive,password",
                &host_with_user,
                "bash",
                "-c",
                &move_config_cmd,
            ]);
        }
        move_config.stdout(Stdio::null());
        move_config.stderr(Stdio::inherit());
        let move_config_status = move_config.status()?;
        if !move_config_status.success() {
            anyhow::bail!("Failed to move ca-montreal.ovpn to $HOME/config/vpn");
        }
        println!("✓ Copied ca-montreal.ovpn to remote system");
    }

    // Copy compose file to remote system (keep in home directory for user access)
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
    println!(
        "    - /home/{}/config/vpn/auth.txt (OpenVPN authentication)",
        vpn_user
    );
    println!(
        "    - /home/{}/config/vpn/ca-montreal.ovpn (OpenVPN configuration)",
        vpn_user
    );
    println!();
    println!("  Note: Set USER environment variable in Portainer to match the username");
    println!("        Example: USER={}", vpn_user);
    println!();
    println!("  You can now deploy the VPN manually using Portainer or docker-compose.");

    Ok(())
}
