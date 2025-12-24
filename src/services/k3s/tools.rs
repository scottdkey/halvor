//! Tool installation functions for K3s cluster setup
use crate::services::k3s::utils;
use crate::utils::exec::{CommandExecutor, PackageManager};
use anyhow::{Context, Result};
use reqwest;
use std::io::Write;

/// Install halvor on a remote machine
/// In development mode: copies local binary to remote
/// In production mode: downloads from GitHub releases
/// If halvor exists but is outdated, replaces it with the newer version
pub fn check_and_install_halvor<E: CommandExecutor>(exec: &E) -> Result<()> {
    let is_dev = utils::is_development_mode();
    let local_version = env!("CARGO_PKG_VERSION");

    // Check if halvor is already installed
    if exec.check_command_exists("halvor")? {
        // Get remote version
        let remote_version_output = exec
            .execute_shell("halvor --version 2>&1 | head -1 || echo 'unknown'")
            .ok();
        let remote_version = remote_version_output
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| {
                // Extract version from output like "halvor 1.2.3 (experimental)"
                s.trim()
                    .strip_prefix("halvor ")
                    .and_then(|v| v.split_whitespace().next())
                    .unwrap_or("unknown")
                    .to_string()
            })
            .unwrap_or_else(|| "unknown".to_string());

        // In dev mode, always replace to ensure latest code
        // In production, only replace if versions differ
        if is_dev {
            println!(
                "halvor found (version: {}), replacing with local version ({})...",
                remote_version, local_version
            );
        } else if remote_version != local_version && remote_version != "unknown" {
            println!(
                "halvor found (version: {}), replacing with newer version ({})...",
                remote_version, local_version
            );
        } else {
            println!(
                "✓ halvor is already installed (version: {})",
                remote_version
            );
            return Ok(());
        }
    } else {
        println!("halvor not found, installing...");
    }

    // Detect remote platform first (needed for both dev and production)
    let arch_output = exec.execute_shell("uname -m")?;
    let arch_bytes = arch_output.stdout;
    let arch_str = String::from_utf8_lossy(&arch_bytes).trim().to_string();
    let remote_arch = match arch_str.as_str() {
        "x86_64" | "amd64" => "amd64",
        "aarch64" | "arm64" => "arm64",
        _ => anyhow::bail!("Unsupported architecture: {}", arch_str),
    };

    let os_output = exec.execute_shell("uname -s")?;
    let os_bytes = os_output.stdout;
    let os_str = String::from_utf8_lossy(&os_bytes).trim().to_string();
    let remote_os = match os_str.as_str() {
        "Linux" => "linux",
        "Darwin" => "darwin",
        _ => anyhow::bail!("Unsupported OS: {}", os_str),
    };

    // Check for musl (Alpine)
    let musl_check = exec
        .execute_shell("ldd --version 2>&1 | grep -q musl && echo musl || echo glibc")
        .ok();
    let is_musl = musl_check
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().contains("musl"))
        .unwrap_or(false);

    let remote_platform = if is_musl && remote_os == "linux" {
        format!("{}-{}-musl", remote_os, remote_arch)
    } else {
        format!("{}-{}", remote_os, remote_arch)
    };

    // Always download from GitHub releases
    // In development mode, use "experimental" release
    // In production mode, use "latest" release
    if is_dev {
        println!("  Development mode: downloading halvor from GitHub 'experimental' release...");
        download_halvor_from_github(exec, &remote_platform, "experimental")?;
    } else {
        println!("  Production mode: downloading halvor from GitHub 'latest' release...");
        download_halvor_from_github(exec, &remote_platform, "latest")?;
    }

    // Verify installation
    if !exec.check_command_exists("halvor")? {
        anyhow::bail!("halvor installation completed but halvor command is not available.");
    }

    println!("✓ halvor installed successfully");
    Ok(())
}

/// Download and install halvor from GitHub releases for a specific platform
/// release_tag: "latest" for production, "experimental" for development
fn download_halvor_from_github<E: CommandExecutor>(
    exec: &E,
    platform: &str,
    release_tag: &str,
) -> Result<()> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("Failed to create HTTP client")?;

    // Get release from GitHub API
    let github_api = if release_tag == "latest" {
        "https://api.github.com/repos/scottdkey/halvor/releases/latest".to_string()
    } else {
        format!(
            "https://api.github.com/repos/scottdkey/halvor/releases/tags/{}",
            release_tag
        )
    };

    let release_json: serde_json::Value = client
        .get(&github_api)
        .send()
        .context(format!(
            "Failed to fetch {} release from GitHub",
            release_tag
        ))?
        .error_for_status()
        .context(format!("HTTP error fetching {} release", release_tag))?
        .json()
        .context("Failed to parse release JSON")?;

    // Find the asset for this platform
    // Asset names are like: halvor-{version}-{platform}.tar.gz
    // For experimental, we need to match any version
    let assets = release_json
        .get("assets")
        .and_then(|a| a.as_array())
        .context("No assets found in release")?;

    let asset_url = assets
        .iter()
        .find_map(|asset| {
            let name = asset.get("name")?.as_str()?;
            let download_url = asset.get("browser_download_url")?.as_str()?;

            // Match platform in asset name (e.g., halvor-*-linux-amd64.tar.gz or halvor-*-darwin-arm64.tar.gz)
            if name.contains(platform) && (name.ends_with(".tar.gz") || name.ends_with(".zip")) {
                Some(download_url)
            } else {
                None
            }
        })
        .context(format!(
            "No release asset found for platform: {} in {} release",
            platform, release_tag
        ))?;

    // Download the tarball
    println!("  Downloading from: {}", asset_url);
    let tarball = client
        .get(asset_url)
        .send()
        .context("Failed to download halvor release")?
        .error_for_status()
        .context("HTTP error downloading halvor release")?
        .bytes()
        .context("Failed to read tarball content")?;

    // Write tarball to remote using scp (via write_file)
    let remote_tarball = "/tmp/halvor.tar.gz";
    exec.write_file(remote_tarball, &tarball)
        .context("Failed to write tarball to remote host")?;

    // Extract and install
    exec.execute_shell_interactive(&format!(
        "cd /tmp && tar -xzf {} && sudo mv halvor /usr/local/bin/halvor && chmod +x /usr/local/bin/halvor && rm -f {}",
        remote_tarball, remote_tarball
    ))
    .context("Failed to extract and install halvor")?;

    Ok(())
}

/// Check if kubectl is installed and install it if not
pub fn check_and_install_kubectl<E: CommandExecutor>(exec: &E) -> Result<()> {
    if exec.check_command_exists("kubectl")? {
        println!("✓ kubectl is already installed");
        return Ok(());
    }

    println!("kubectl not found, installing...");

    // Detect package manager
    let pkg_mgr = PackageManager::detect(exec).context("Failed to detect package manager")?;

    match pkg_mgr {
        PackageManager::Apt => {
            // For Debian/Ubuntu, add Kubernetes repo and install
            println!("  Detected apt - installing kubectl from Kubernetes repository");

            // Download GPG key using reqwest
            let gpg_key_url = "https://pkgs.k8s.io/core:/stable:/v1.28/deb/Release.key";
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .context("Failed to create HTTP client")?;

            let gpg_key = client
                .get(gpg_key_url)
                .send()
                .context("Failed to download Kubernetes GPG key")?
                .error_for_status()
                .context("HTTP error downloading Kubernetes GPG key")?
                .bytes()
                .context("Failed to read GPG key content")?;

            let temp_key_path = "/tmp/kubernetes-gpg-key.asc";
            exec.write_file(temp_key_path, &gpg_key)
                .context("Failed to write GPG key to temporary file")?;

            exec.execute_shell_interactive(&format!(
                "sudo gpg --dearmor -o /etc/apt/keyrings/kubernetes-apt-keyring.gpg {}",
                temp_key_path
            ))?;

            exec.execute_shell_interactive("echo 'deb [signed-by=/etc/apt/keyrings/kubernetes-apt-keyring.gpg] https://pkgs.k8s.io/core:/stable:/v1.28/deb/ /' | sudo tee /etc/apt/sources.list.d/kubernetes.list")?;
            exec.execute_shell_interactive("sudo apt-get update")?;
            exec.execute_shell_interactive("sudo apt-get install -y kubectl")?;

            // Clean up temp file
            let _ = exec.execute_shell(&format!("rm -f {}", temp_key_path));
        }
        PackageManager::Yum | PackageManager::Dnf => {
            // For RHEL/CentOS/Fedora, add Kubernetes repo and install
            println!(
                "  Detected {} - installing kubectl from Kubernetes repository",
                pkg_mgr.display_name()
            );
            let install_cmd = format!(
                "cat <<EOF | sudo tee /etc/yum.repos.d/kubernetes.repo
[kubernetes]
name=Kubernetes
baseurl=https://pkgs.k8s.io/core:/stable:/v1.28/rpm/
enabled=1
gpgcheck=1
gpgkey=https://pkgs.k8s.io/core:/stable:/v1.28/rpm/repodata/repomd.xml.asc
EOF"
            );
            exec.execute_shell_interactive(&install_cmd)?;
            if pkg_mgr == PackageManager::Yum {
                exec.execute_shell_interactive("sudo yum install -y kubectl")?;
            } else {
                exec.execute_shell_interactive("sudo dnf install -y kubectl")?;
            }
        }
        PackageManager::Brew => {
            println!("  Detected brew - installing kubectl");
            exec.execute_shell_interactive("brew install kubectl")?;
        }
        PackageManager::Unknown => {
            // Fallback: download binary directly using reqwest
            println!("  No package manager detected - downloading kubectl binary");
            let arch = exec.execute_shell("uname -m")?;
            let arch_bytes = arch.stdout;
            let arch_str = String::from_utf8_lossy(&arch_bytes).trim().to_string();
            let kubectl_arch = match arch_str.as_str() {
                "x86_64" => "amd64",
                "aarch64" | "arm64" => "arm64",
                _ => anyhow::bail!("Unsupported architecture: {}", arch_str),
            };

            let kubectl_url = format!(
                "https://dl.k8s.io/release/v1.28.0/bin/linux/{}/kubectl",
                kubectl_arch
            );

            // Download kubectl binary using reqwest
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .context("Failed to create HTTP client")?;

            let kubectl_binary = client
                .get(&kubectl_url)
                .send()
                .context("Failed to download kubectl binary")?
                .error_for_status()
                .context("HTTP error downloading kubectl binary")?
                .bytes()
                .context("Failed to read kubectl binary content")?;

            let temp_binary_path = "/tmp/kubectl";
            exec.write_file(temp_binary_path, &kubectl_binary)
                .context("Failed to write kubectl binary to temporary file")?;

            exec.execute_shell_interactive(&format!(
                "chmod +x {} && sudo mv {} /usr/local/bin/kubectl",
                temp_binary_path, temp_binary_path
            ))?;
        }
    }

    println!("✓ kubectl installed successfully");
    Ok(())
}

/// Check if helm is installed and install it if not
pub fn check_and_install_helm<E: CommandExecutor>(exec: &E) -> Result<()> {
    if exec.check_command_exists("helm")? {
        println!("✓ helm is already installed");
        return Ok(());
    }

    println!("helm not found, installing...");

    // Download Helm install script using reqwest
    println!("  Downloading Helm install script from GitHub...");
    let script_url = "https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3";
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60)) // Increased timeout for slow connections
        .build()
        .context("Failed to create HTTP client")?;

    println!("  Fetching script content...");
    let script_content = client
        .get(script_url)
        .send()
        .context("Failed to download Helm install script")?
        .error_for_status()
        .context("HTTP error downloading Helm install script")?
        .text()
        .context("Failed to read Helm install script content")?;

    println!("  Script downloaded ({} bytes)", script_content.len());

    // Write script to remote host and execute
    let remote_script_path = "/tmp/get-helm-3.sh";
    println!("  Writing script to remote host...");
    exec.write_file(remote_script_path, script_content.as_bytes())
        .context("Failed to write Helm install script to remote host")?;

    // Make script executable using execute_simple (passes arguments directly, avoids shell parsing)
    println!("  Making script executable...");
    let chmod_output = exec.execute_simple("chmod", &["+x", remote_script_path])?;
    if !chmod_output.status.success() {
        anyhow::bail!(
            "Failed to make Helm install script executable: {}",
            String::from_utf8_lossy(&chmod_output.stderr)
        );
    }

    // Verify script exists and is executable before attempting to run
    let script_exists = exec.file_exists(remote_script_path)?;
    if !script_exists {
        anyhow::bail!("Helm install script was not written correctly - file does not exist");
    }

    // Execute the script - it uses sudo, so we need interactive mode
    println!("  Executing Helm install script (this may take a minute)...");
    println!("  Note: The script may prompt for sudo password");
    println!("  Script path: {}", remote_script_path);

    // Flush stdout to ensure messages are displayed before script execution
    std::io::stdout().flush().ok();

    // The Helm install script downloads and installs Helm, which requires sudo
    // We must use execute_shell_interactive to allow sudo password prompts
    // IMPORTANT: The script must be executed with an explicit shell command
    // Using execute_interactive with bash directly to ensure it runs
    exec.execute_interactive("bash", &[remote_script_path])
        .context(
            "Failed to execute Helm install script. The script may have timed out or required input. \
             If the script appears to hang, it may be waiting for sudo password input.",
        )?;

    // Clean up script
    let _ = exec.execute_shell(&format!("rm -f {}", remote_script_path));

    // Verify helm was actually installed
    if !exec.check_command_exists("helm")? {
        anyhow::bail!(
            "Helm installation completed but helm command is not available. Installation may have failed."
        );
    }

    println!("✓ helm installed successfully");
    Ok(())
}
