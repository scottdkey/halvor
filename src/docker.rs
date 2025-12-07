use crate::exec::CommandExecutor;
use anyhow::{Context, Result};
use serde_json::{Value, json};

/// Check if Docker is installed and install it if not
pub fn check_and_install<E: CommandExecutor>(exec: &E) -> Result<()> {
    println!("=== Checking Docker installation ===");

    if exec.check_command_exists("docker")? {
        println!("✓ Docker already installed");
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

/// Install Docker GPG key using curl (works with any CommandExecutor)
fn install_gpg_key<E: CommandExecutor>(exec: &E, url: &str) -> Result<()> {
    // Use curl to download and process the key in one command
    let curl_cmd = format!(
        "curl -fsSL {} | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg",
        url
    );
    let output = exec.execute_shell(&curl_cmd)?;
    if !output.status.success() {
        anyhow::bail!("Failed to download and install Docker GPG key");
    }
    exec.execute_interactive("sudo", &["chmod", "a+r", "/etc/apt/keyrings/docker.gpg"])?;
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
    } else {
        println!("✓ User already in docker group");
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

        // Try Python3 first
        if exec.check_command_exists("python3")? {
            let python_script = format!(
                r#"import json; f=open('/etc/docker/daemon.json','r'); c=json.load(f); f.close(); c['ipv6']=True; c['fixed-cidr-v6']='{}'; f=open('/etc/docker/daemon.json','w'); json.dump(c,f,indent=2); f.close()"#,
                ipv6_subnet
            );
            exec.execute_interactive("sudo", &["python3", "-c", &python_script])?;
        } else if exec.check_command_exists("jq")? {
            // Use Rust-native JSON manipulation
            update_daemon_json_rust(exec, ipv6_subnet)?;
        } else {
            // Fallback: backup and create new
            println!(
                "Warning: python3/jq not found, backing up existing config and creating new one"
            );
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

    println!("✓ IPv6 configured in Docker daemon");
    println!("Restarting Docker daemon to apply changes...");

    if exec.check_command_exists("systemctl")? {
        exec.execute_interactive("sudo", &["systemctl", "restart", "docker"])?;
    } else if exec.check_command_exists("service")? {
        exec.execute_interactive("sudo", &["service", "docker", "restart"])?;
    } else {
        println!(
            "Warning: Could not restart Docker daemon. Please restart manually: sudo systemctl restart docker"
        );
    }

    // Wait a moment
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Verify IPv6 is enabled
    let verify_output = exec.execute_simple("docker", &["info"])?;
    let docker_info = String::from_utf8_lossy(&verify_output.stdout);
    if docker_info.to_lowercase().contains("ipv6") && docker_info.to_lowercase().contains("true") {
        println!("✓ IPv6 verified in Docker");
    } else {
        println!("Warning: IPv6 may not be enabled. Check with: docker info | grep -i ipv6");
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

    // Write updated config
    let updated_content = serde_json::to_string_pretty(&config)?;
    exec.write_file("/tmp/daemon.json", updated_content.as_bytes())?;
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
