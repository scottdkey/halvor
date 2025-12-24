use crate::config::EnvConfig;
use crate::utils::exec::{CommandExecutor, Executor};
use anyhow::{Context, Result};
use reqwest;
use serde_json::{Value, json};

pub mod build;
pub mod diagnostics;

/// Check if Docker is installed
pub fn is_docker_installed<E: CommandExecutor>(exec: &E) -> bool {
    exec.check_command_exists("docker").unwrap_or(false)
}

/// Check if Docker daemon is running and start it if needed
pub fn ensure_docker_running<E: CommandExecutor>(exec: &E) -> Result<()> {
    // Try to run a simple docker command to check if daemon is accessible
    let check_output = exec.execute_simple("docker", &["info"]);

    if let Ok(output) = check_output {
        if output.status.success() {
            return Ok(()); // Docker is running and accessible
        }
    }

    // Docker daemon might not be running, try to start it
    println!("Docker daemon not accessible, attempting to start...");

    if exec.check_command_exists("systemctl")? {
        // Check if docker service exists
        let status_output = exec.execute_simple("systemctl", &["is-active", "docker"]);
        if let Ok(output) = status_output {
            let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if status == "inactive" || status == "failed" {
                println!("Starting Docker daemon...");
                exec.execute_interactive("sudo", &["systemctl", "start", "docker"])?;
                exec.execute_interactive("sudo", &["systemctl", "enable", "docker"])?;
                // Wait a moment for Docker to start
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
        } else {
            // Service might not exist, try to start anyway
            exec.execute_interactive("sudo", &["systemctl", "start", "docker"])
                .ok();
        }
    } else if exec.check_command_exists("service")? {
        exec.execute_interactive("sudo", &["service", "docker", "start"])
            .ok();
        std::thread::sleep(std::time::Duration::from_secs(2));
    }

    // Verify Docker is now accessible
    let verify_output = exec.execute_simple("docker", &["info"]);
    match verify_output {
        Ok(output) if output.status.success() => {
            println!("✓ Docker daemon is running");
            Ok(())
        }
        _ => {
            // If still not accessible, it might be a permissions issue
            // Try with sudo to verify daemon is running
            let sudo_check = exec.execute_simple("sudo", &["docker", "info"]);
            if let Ok(output) = sudo_check {
                if output.status.success() {
                    println!("⚠ Docker daemon is running but user doesn't have access");
                    println!("⚠ User may need to be added to docker group or use sudo");
                    // Don't fail here, let configure_permissions handle it
                    return Ok(());
                }
            }
            anyhow::bail!(
                "Docker daemon is not running or not accessible. Please start it manually: sudo systemctl start docker"
            )
        }
    }
}

/// Check if Docker is installed and install it if not
pub fn check_and_install<E: CommandExecutor>(exec: &E) -> Result<()> {
    println!("=== Checking Docker installation ===");

    if exec.check_command_exists("docker")? {
        println!("✓ Docker already installed");
        // Ensure daemon is running
        ensure_docker_running(exec)?;
        return Ok(());
    }

    // For local execution, just provide instructions
    // Check if this is a local executor by checking if it's the Executor::Local variant
    // We can't easily check this with generics, so we'll use a different approach
    // For now, we'll check if the executor can read /etc/os-release (remote) or not
    let is_local_executor = exec.read_file("/etc/os-release").is_err();

    if is_local_executor {
        println!("Docker not found. Please install Docker manually.");
        println!("  Linux: https://docs.docker.com/engine/install/");
        println!("  macOS: https://docs.docker.com/desktop/install/mac-install/");
        println!("  Windows: https://docs.docker.com/desktop/install/windows-install/");
        anyhow::bail!("Docker installation required");
    }

    // For remote execution, install Docker automatically
    println!("Docker not found. Installing Docker...");

    // Detect OS type
    let os_release_output = exec.read_file("/etc/os-release")?;
    let is_debian = os_release_output
        .lines()
        .any(|line| line.starts_with("ID=debian") || line.starts_with("ID=\"debian\""));

    if exec.check_command_exists("apt-get")? {
        if is_debian {
            install_debian(exec)?;
        } else {
            install_ubuntu(exec)?;
        }

        exec.execute_interactive("sudo", &["apt-get", "update"])?;
        exec.execute_interactive(
            "sudo",
            &[
                "apt-get",
                "install",
                "-y",
                "docker-ce",
                "docker-ce-cli",
                "containerd.io",
                "docker-buildx-plugin",
                "docker-compose-plugin",
            ],
        )?;
    } else if exec.check_command_exists("yum")? {
        install_rhel_centos(exec)?;
    } else if exec.check_command_exists("dnf")? {
        install_fedora(exec)?;
    } else if exec.check_command_exists("brew")? {
        println!("Detected macOS");
        exec.execute_interactive("brew", &["install", "--cask", "docker"])?;
        println!("Please start Docker Desktop manually");
    } else {
        anyhow::bail!("Unsupported package manager. Please install Docker manually.");
    }

    println!("✓ Docker installed");
    Ok(())
}

fn install_debian<E: CommandExecutor>(exec: &E) -> Result<()> {
    println!("Detected Debian, using Debian Docker repository");
    exec.execute_interactive("sudo", &["rm", "-f", "/etc/apt/sources.list.d/docker.list"])?;
    exec.execute_interactive("sudo", &["apt-get", "update"])?;
    exec.execute_interactive(
        "sudo",
        &[
            "apt-get",
            "install",
            "-y",
            "ca-certificates",
            "curl",
            "gnupg",
        ],
    )?;
    exec.execute_interactive(
        "sudo",
        &["install", "-m", "0755", "-d", "/etc/apt/keyrings"],
    )?;

    // Download and install GPG key
    install_gpg_key(exec, "https://download.docker.com/linux/debian/gpg")?;

    // Get codename using native Rust (read and parse file)
    let codename = if let Ok(os_release) = exec.read_file("/etc/os-release") {
        os_release
            .lines()
            .find(|line| line.starts_with("VERSION_CODENAME="))
            .and_then(|line| line.split('=').nth(1))
            .map(|s| s.trim_matches('"').to_string())
            .unwrap_or_else(|| "bookworm".to_string())
    } else {
        "bookworm".to_string()
    };

    // Get architecture
    let arch_output = exec.execute_simple("dpkg", &["--print-architecture"])?;
    let arch = String::from_utf8_lossy(&arch_output.stdout)
        .trim()
        .to_string();

    let repo_line = format!(
        "deb [arch={} signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/debian {} stable",
        arch, codename
    );
    exec.write_file("/tmp/docker.list", repo_line.as_bytes())?;
    exec.execute_interactive(
        "sudo",
        &[
            "mv",
            "/tmp/docker.list",
            "/etc/apt/sources.list.d/docker.list",
        ],
    )?;
    Ok(())
}

fn install_ubuntu<E: CommandExecutor>(exec: &E) -> Result<()> {
    println!("Detected Ubuntu, using Ubuntu Docker repository");
    exec.execute_interactive("sudo", &["rm", "-f", "/etc/apt/sources.list.d/docker.list"])?;
    exec.execute_interactive("sudo", &["apt-get", "update"])?;
    exec.execute_interactive(
        "sudo",
        &[
            "apt-get",
            "install",
            "-y",
            "ca-certificates",
            "curl",
            "gnupg",
            "lsb-release",
        ],
    )?;
    exec.execute_interactive(
        "sudo",
        &["install", "-m", "0755", "-d", "/etc/apt/keyrings"],
    )?;

    // Download and install GPG key
    install_gpg_key(exec, "https://download.docker.com/linux/ubuntu/gpg")?;

    // Get codename
    let codename_output = exec.execute_simple("lsb_release", &["-cs"])?;
    let codename = String::from_utf8_lossy(&codename_output.stdout)
        .trim()
        .to_string();

    // Get architecture
    let arch_output = exec.execute_simple("dpkg", &["--print-architecture"])?;
    let arch = String::from_utf8_lossy(&arch_output.stdout)
        .trim()
        .to_string();

    let repo_line = format!(
        "deb [arch={} signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu {} stable",
        arch, codename
    );
    exec.write_file("/tmp/docker.list", repo_line.as_bytes())?;
    exec.execute_interactive(
        "sudo",
        &[
            "mv",
            "/tmp/docker.list",
            "/etc/apt/sources.list.d/docker.list",
        ],
    )?;
    Ok(())
}

fn install_rhel_centos<E: CommandExecutor>(exec: &E) -> Result<()> {
    println!("Detected RHEL/CentOS");
    exec.execute_interactive("sudo", &["yum", "install", "-y", "yum-utils"])?;
    exec.execute_interactive(
        "sudo",
        &[
            "yum-config-manager",
            "--add-repo",
            "https://download.docker.com/linux/centos/docker-ce.repo",
        ],
    )?;
    exec.execute_interactive(
        "sudo",
        &[
            "yum",
            "install",
            "-y",
            "docker-ce",
            "docker-ce-cli",
            "containerd.io",
            "docker-buildx-plugin",
            "docker-compose-plugin",
        ],
    )?;
    exec.execute_interactive("sudo", &["systemctl", "start", "docker"])?;
    exec.execute_interactive("sudo", &["systemctl", "enable", "docker"])?;
    Ok(())
}

fn install_fedora<E: CommandExecutor>(exec: &E) -> Result<()> {
    println!("Detected Fedora");
    exec.execute_interactive("sudo", &["dnf", "install", "-y", "dnf-plugins-core"])?;
    exec.execute_interactive(
        "sudo",
        &[
            "dnf",
            "config-manager",
            "--add-repo",
            "https://download.docker.com/linux/fedora/docker-ce.repo",
        ],
    )?;
    exec.execute_interactive(
        "sudo",
        &[
            "dnf",
            "install",
            "-y",
            "docker-ce",
            "docker-ce-cli",
            "containerd.io",
            "docker-buildx-plugin",
            "docker-compose-plugin",
        ],
    )?;
    exec.execute_interactive("sudo", &["systemctl", "start", "docker"])?;
    exec.execute_interactive("sudo", &["systemctl", "enable", "docker"])?;
    Ok(())
}

/// Install Docker GPG key using native Rust HTTP client
fn install_gpg_key<E: CommandExecutor>(exec: &E, url: &str) -> Result<()> {
    // Download GPG key using native Rust HTTP client
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("Failed to create HTTP client")?;

    let key_content = client
        .get(url)
        .send()
        .context("Failed to download Docker GPG key")?
        .error_for_status()
        .context("HTTP error downloading Docker GPG key")?
        .bytes()
        .context("Failed to read Docker GPG key content")?;

    // Write key to temporary file, then use gpg to process it
    let temp_key_path = "/tmp/docker-gpg-key.asc";
    exec.write_file(temp_key_path, &key_content)
        .context("Failed to write GPG key to temporary file")?;

    // Process the key with gpg and install it
    let gpg_cmd = format!(
        "sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg {}",
        temp_key_path
    );
    let output = exec.execute_shell(&gpg_cmd)?;
    if !output.status.success() {
        anyhow::bail!("Failed to process and install Docker GPG key");
    }

    exec.execute_interactive("sudo", &["chmod", "a+r", "/etc/apt/keyrings/docker.gpg"])?;

    // Clean up temporary file
    let _ = exec.execute_shell(&format!("rm -f {}", temp_key_path));

    Ok(())
}

/// Configure Docker permissions (works for both local and remote)
pub fn configure_permissions<E: CommandExecutor>(exec: &E) -> Result<()> {
    println!();
    println!("=== Configuring Docker permissions ===");

    if !exec.is_linux()? {
        return Ok(());
    }

    // Check if user is in docker group using native Rust (Unix only)
    let username = exec.get_username()?;
    #[cfg(unix)]
    let in_group = {
        use std::fs;
        if let Ok(group_content) = fs::read_to_string("/etc/group") {
            group_content
                .lines()
                .any(|line| line.starts_with("docker:") && line.contains(&username))
        } else {
            // Fallback to groups command if file read fails
            let groups_output = exec.execute_simple("groups", &[])?;
            let groups = String::from_utf8_lossy(&groups_output.stdout);
            groups.contains("docker")
        }
    };
    #[cfg(not(unix))]
    let in_group = {
        // On non-Unix, use groups command
        let groups_output = exec.execute_simple("groups", &[])?;
        let groups = String::from_utf8_lossy(&groups_output.stdout);
        groups.contains("docker")
    };

    if !in_group {
        println!("Adding user to docker group...");
        exec.execute_interactive("sudo", &["usermod", "-aG", "docker", &username])?;
        println!("✓ User added to docker group");
        println!("Note: You may need to log out and back in for changes to take effect");

        // Try to apply group changes immediately using newgrp or by checking if we can use docker
        // For SSH sessions, we can't easily apply group changes, but we can try using newgrp
        // However, this is complex, so we'll just note it and continue
        // The user can use 'newgrp docker' or restart their session
    } else {
        println!("✓ User already in docker group");
    }

    // Verify Docker access after group configuration
    // If user was just added, they might need to use 'newgrp docker' or restart session
    // But we can try to verify with a simple command
    let test_output =
        exec.execute_simple("docker", &["version", "--format", "{{.Server.Version}}"]);
    match test_output {
        Ok(output) if output.status.success() => {
            let _version = String::from_utf8_lossy(&output.stdout);
            println!("✓ Docker access verified");
        }
        _ => {
            // Try with newgrp docker if available, otherwise warn
            println!(
                "⚠ Docker command failed - user may need to run 'newgrp docker' or restart SSH session"
            );
            println!("⚠ Alternatively, using 'sudo docker' commands will work");
        }
    }

    Ok(())
}

/// Configure Docker IPv6 support (works for both local and remote)
pub fn configure_ipv6<E: CommandExecutor>(exec: &E) -> Result<()> {
    println!();
    println!("=== Configuring Docker IPv6 support ===");

    if !exec.is_linux()? {
        println!(
            "Skipping IPv6 configuration (macOS/Windows - Docker Desktop handles IPv6 differently)"
        );
        return Ok(());
    }

    let ipv6_subnet = "fd00:172:20::/64";
    let daemon_file = "/etc/docker/daemon.json";

    // Check if IPv6 is already enabled
    let ipv6_enabled = if exec.file_exists(daemon_file)? {
        let content = exec.read_file(daemon_file)?;
        content.contains("\"ipv6\"") && content.contains("true")
    } else {
        false
    };

    if ipv6_enabled {
        println!("✓ IPv6 already enabled in Docker daemon");
        return Ok(());
    }

    println!("Configuring IPv6 in Docker daemon...");

    // Create directory if needed
    exec.execute_interactive("sudo", &["mkdir", "-p", "/etc/docker"])?;

    // Check if daemon.json exists
    let exists = exec.file_exists(daemon_file)?;

    if !exists {
        // Create new daemon.json
        println!("Creating new Docker daemon configuration...");
        let config = json!({
            "ipv6": true,
            "fixed-cidr-v6": ipv6_subnet
        });
        let config_str = serde_json::to_string_pretty(&config)?;
        exec.write_file("/tmp/daemon.json", config_str.as_bytes())?;
        exec.execute_interactive(
            "sudo",
            &["mv", "/tmp/daemon.json", "/etc/docker/daemon.json"],
        )?;
    } else {
        // Update existing daemon.json
        println!("Updating existing Docker daemon configuration...");

        // Use Rust-native JSON manipulation (more reliable than Python via SSH)
        match update_daemon_json_rust(exec, ipv6_subnet) {
            Ok(_) => {
                println!("✓ Docker daemon configuration updated");
            }
            Err(e) => {
                // Fallback: backup and create new if Rust method fails
                println!("Warning: Failed to update existing config: {}", e);
                println!("Backing up existing config and creating new one...");
                exec.execute_interactive(
                    "sudo",
                    &[
                        "cp",
                        "/etc/docker/daemon.json",
                        "/etc/docker/daemon.json.backup",
                    ],
                )?;
                let config = json!({
                    "ipv6": true,
                    "fixed-cidr-v6": ipv6_subnet
                });
                let config_str = serde_json::to_string_pretty(&config)?;
                exec.write_file("/tmp/daemon.json", config_str.as_bytes())?;
                exec.execute_interactive(
                    "sudo",
                    &["mv", "/tmp/daemon.json", "/etc/docker/daemon.json"],
                )?;
                println!("Original config backed up to /etc/docker/daemon.json.backup");
            }
        }
    }

    println!("✓ IPv6 configured in Docker daemon");
    println!("Restarting Docker daemon to apply changes...");

    let restart_result = if exec.check_command_exists("systemctl")? {
        exec.execute_interactive("sudo", &["systemctl", "restart", "docker"])
    } else if exec.check_command_exists("service")? {
        exec.execute_interactive("sudo", &["service", "docker", "restart"])
    } else {
        println!(
            "Warning: Could not restart Docker daemon. Please restart manually: sudo systemctl restart docker"
        );
        return Ok(());
    };

    match restart_result {
        Ok(_) => {
            println!("✓ Docker daemon restarted successfully");
        }
        Err(e) => {
            println!("Error: Failed to restart Docker daemon: {}", e);
            println!();
            println!("This may indicate a syntax error in /etc/docker/daemon.json");
            println!("To diagnose:");
            println!("  1. Check Docker daemon status: sudo systemctl status docker.service");
            println!("  2. Check Docker logs: sudo journalctl -xeu docker.service");
            println!(
                "  3. Validate daemon.json: sudo python3 -m json.tool /etc/docker/daemon.json"
            );
            println!(
                "  4. If needed, restore backup: sudo cp /etc/docker/daemon.json.backup /etc/docker/daemon.json"
            );
            return Err(e).context("Docker daemon restart failed - check daemon.json syntax");
        }
    }

    // Wait a moment for Docker to start
    std::thread::sleep(std::time::Duration::from_secs(3));

    // Verify Docker is running and IPv6 is enabled
    let verify_output = exec.execute_simple("docker", &["info"]);
    match verify_output {
        Ok(output) => {
            let docker_info = String::from_utf8_lossy(&output.stdout);
            if docker_info.to_lowercase().contains("ipv6")
                && docker_info.to_lowercase().contains("true")
            {
                println!("✓ IPv6 verified in Docker");
            } else {
                println!(
                    "Warning: IPv6 may not be enabled. Check with: docker info | grep -i ipv6"
                );
            }
        }
        Err(e) => {
            println!("Warning: Could not verify Docker status: {}", e);
            println!("Docker may still be starting up. Check with: docker info");
        }
    }

    Ok(())
}

/// Generic version of update_daemon_json_rust that works with any CommandExecutor
fn update_daemon_json_rust<E: CommandExecutor>(exec: &E, ipv6_subnet: &str) -> Result<()> {
    // Read existing config
    let content = exec.read_file("/etc/docker/daemon.json")?;
    let mut config: Value = serde_json::from_str(&content)
        .with_context(|| "Failed to parse /etc/docker/daemon.json")?;

    // Update config
    config["ipv6"] = json!(true);
    config["fixed-cidr-v6"] = json!(ipv6_subnet);

    // Validate JSON before writing
    let updated_content = serde_json::to_string_pretty(&config)?;

    // Verify the JSON is valid by parsing it back
    serde_json::from_str::<Value>(&updated_content)
        .with_context(|| "Generated invalid JSON for daemon.json")?;

    exec.write_file("/tmp/daemon.json", updated_content.as_bytes())?;

    // Validate the file can be read back as JSON before moving it
    let verify_content = exec.read_file("/tmp/daemon.json")?;
    serde_json::from_str::<Value>(&verify_content)
        .with_context(|| "Written file contains invalid JSON")?;

    exec.execute_interactive(
        "sudo",
        &["mv", "/tmp/daemon.json", "/etc/docker/daemon.json"],
    )?;

    Ok(())
}

/// Stop all running Docker containers
pub fn stop_all_containers<E: CommandExecutor>(exec: &E) -> Result<Vec<String>> {
    // Get running containers
    let running_output = exec.execute_simple("docker", &["ps", "-q"])?;
    let running_containers = String::from_utf8_lossy(&running_output.stdout);
    let running_containers: Vec<&str> = running_containers
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();

    if !running_containers.is_empty() {
        let container_ids: Vec<String> = running_containers.iter().map(|s| s.to_string()).collect();
        let stop_cmd = format!("docker stop {}", container_ids.join(" "));
        let stop_output = exec.execute_shell(&stop_cmd)?;
        if !stop_output.status.success() {
            // Try with sudo
            let sudo_stop =
                exec.execute_shell(&format!("sudo docker stop {}", container_ids.join(" ")))?;
            if !sudo_stop.status.success() {
                anyhow::bail!("Failed to stop containers");
            }
        }
        Ok(container_ids)
    } else {
        Ok(Vec::new())
    }
}

/// Start Docker containers by their IDs
pub fn start_containers<E: CommandExecutor>(exec: &E, container_ids: &[String]) -> Result<()> {
    if !container_ids.is_empty() {
        let start_cmd = format!("docker start {}", container_ids.join(" "));
        let start_output = exec.execute_shell(&start_cmd)?;
        if !start_output.status.success() {
            let sudo_start =
                exec.execute_shell(&format!("sudo docker start {}", container_ids.join(" ")))?;
            if !sudo_start.status.success() {
                anyhow::bail!("Failed to start containers");
            }
        }
    }
    Ok(())
}

/// Get all Docker volumes
pub fn list_volumes<E: CommandExecutor>(exec: &E) -> Result<Vec<String>> {
    let volumes_output =
        exec.execute_simple("docker", &["volume", "ls", "--format", "{{.Name}}"])?;
    let volumes_str = String::from_utf8_lossy(&volumes_output.stdout);
    let volumes: Vec<String> = volumes_str
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|s| s.to_string())
        .collect();
    Ok(volumes)
}

/// Backup a Docker volume
pub fn backup_volume<E: CommandExecutor>(exec: &E, volume: &str, backup_dir: &str) -> Result<()> {
    let backup_cmd = format!(
        "docker run --rm -v {}:/data:ro -v {}:/backup alpine tar czf /backup/{}.tar.gz -C /data .",
        volume, backup_dir, volume
    );
    let backup_output = exec.execute_shell(&backup_cmd)?;
    if backup_output.status.success() {
        Ok(())
    } else {
        // Try with sudo
        let sudo_backup_cmd = format!(
            "sudo docker run --rm -v {}:/data:ro -v {}:/backup alpine tar czf /backup/{}.tar.gz -C /data .",
            volume, backup_dir, volume
        );
        let sudo_output = exec.execute_shell(&sudo_backup_cmd)?;
        if sudo_output.status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to backup volume: {}", volume)
        }
    }
}

/// Restore a Docker volume
pub fn restore_volume<E: CommandExecutor>(exec: &E, volume: &str, backup_dir: &str) -> Result<()> {
    // Check if volume exists, create if not
    let inspect_output = exec.execute_simple("docker", &["volume", "inspect", volume])?;
    if !inspect_output.status.success() {
        // Volume doesn't exist, create it
        let create_output = exec.execute_simple("docker", &["volume", "create", volume])?;
        if !create_output.status.success() {
            let sudo_create =
                exec.execute_shell(&format!("sudo docker volume create {}", volume))?;
            if !sudo_create.status.success() {
                anyhow::bail!("Failed to create volume: {}", volume);
            }
        }
    }

    // Restore the volume
    let restore_cmd = format!(
        "docker run --rm -v {}:/data -v {}:/backup alpine sh -c 'cd /data && rm -rf * && tar xzf /backup/{}.tar.gz'",
        volume, backup_dir, volume
    );
    let restore_output = exec.execute_shell(&restore_cmd)?;
    if restore_output.status.success() {
        Ok(())
    } else {
        // Try with sudo
        let sudo_restore_cmd = format!(
            "sudo docker run --rm -v {}:/data -v {}:/backup alpine sh -c 'cd /data && rm -rf * && tar xzf /backup/{}.tar.gz'",
            volume, backup_dir, volume
        );
        let sudo_output = exec.execute_shell(&sudo_restore_cmd)?;
        if sudo_output.status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to restore volume: {}", volume)
        }
    }
}

/// Get bind mounts from a container
pub fn get_bind_mounts<E: CommandExecutor>(exec: &E, container: &str) -> Result<Vec<String>> {
    let inspect_cmd = format!(
        r#"docker inspect {} --format '{{{{range .Mounts}}}}{{{{if eq .Type "bind"}}}}{{{{.Source}}}}{{{{end}}}}{{{{end}}}}'"#,
        container
    );
    let mounts_output = exec.execute_shell(&inspect_cmd)?;
    let mounts_str = String::from_utf8_lossy(&mounts_output.stdout);
    let mounts: Vec<String> = mounts_str
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|s| s.to_string())
        .collect();
    Ok(mounts)
}

/// Get all containers
pub fn list_containers<E: CommandExecutor>(exec: &E) -> Result<Vec<String>> {
    let containers_output =
        exec.execute_simple("docker", &["ps", "-a", "--format", "{{.Names}}"])?;
    let containers_str = String::from_utf8_lossy(&containers_output.stdout);
    let containers: Vec<String> = containers_str
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|s| s.to_string())
        .collect();
    Ok(containers)
}

/// Check if a container is running
pub fn is_container_running<E: CommandExecutor>(exec: &E, container_name: &str) -> Result<bool> {
    let output = exec.execute_simple(
        "docker",
        &[
            "ps",
            "--filter",
            &format!("name={}", container_name),
            "--format",
            "{{.Names}}",
        ],
    )?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.trim().contains(container_name))
}

/// Detect the docker compose command to use
/// Returns "docker compose" (plugin) if available, otherwise "docker-compose" (standalone)
pub fn get_compose_command<E: CommandExecutor>(exec: &E) -> Result<String> {
    // First try "docker compose version" to check if the plugin is available
    if exec.check_command_exists("docker")? {
        let output = exec.execute_simple("docker", &["compose", "version"]);
        if let Ok(output) = output {
            if output.status.success() {
                return Ok("docker compose".to_string());
            }
        }
    }

    // Fall back to standalone docker-compose
    if exec.check_command_exists("docker-compose")? {
        Ok("docker-compose".to_string())
    } else {
        anyhow::bail!("Neither 'docker compose' nor 'docker-compose' is available");
    }
}

/// Stop a single container by name
pub fn stop_container<E: CommandExecutor>(exec: &E, container_name: &str) -> Result<()> {
    let output = exec.execute_simple("docker", &["stop", container_name])?;
    if !output.status.success() {
        // Try with sudo
        let sudo_output = exec.execute_simple("sudo", &["docker", "stop", container_name])?;
        if !sudo_output.status.success() {
            anyhow::bail!("Failed to stop container: {}", container_name);
        }
    }
    Ok(())
}

/// Remove a single container by name
pub fn remove_container<E: CommandExecutor>(exec: &E, container_name: &str) -> Result<()> {
    let output = exec.execute_simple("docker", &["rm", container_name])?;
    if !output.status.success() {
        // Try with sudo
        let sudo_output = exec.execute_simple("sudo", &["docker", "rm", container_name])?;
        if !sudo_output.status.success() {
            anyhow::bail!("Failed to remove container: {}", container_name);
        }
    }
    Ok(())
}

/// Stop and remove a container by name (convenience function)
pub fn stop_and_remove_container<E: CommandExecutor>(exec: &E, container_name: &str) -> Result<()> {
    // Stop first (ignore errors if already stopped)
    stop_container(exec, container_name).ok();
    // Then remove
    remove_container(exec, container_name)?;
    Ok(())
}

/// Install Docker on a host (public API for CLI)
pub fn install_docker(hostname: &str, config: &EnvConfig) -> Result<()> {
    let exec = Executor::new(hostname, config)?;
    let target_host = exec.target_host(hostname, config)?;
    let is_local = exec.is_local();

    if is_local {
        println!("Installing Docker locally on {}...", hostname);
    } else {
        println!("Installing Docker on {} ({})...", hostname, target_host);
    }
    println!();

    check_and_install(&exec)?;
    configure_permissions(&exec)?;
    configure_ipv6(&exec)?;

    println!();
    println!("✓ Docker installation complete for {}", hostname);

    Ok(())
}

/// Container definitions for buildable containers
struct ContainerDef {
    name: &'static str,
    build_dir: &'static str,
    image_name: &'static str,
}

/// Registry of buildable containers
const CONTAINERS: &[ContainerDef] = &[ContainerDef {
    name: "pia-vpn",
    build_dir: "openvpn-container",
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
fn find_container_build_path(build_dir: &str) -> Result<std::path::PathBuf> {
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
