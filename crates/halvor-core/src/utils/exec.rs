use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

// Import SshConnection from ssh module
use crate::utils::ssh::SshConnection;
// Agent-based execution is handled in halvor-agent crate

/// Local command execution helpers
pub mod local {
    use super::*;

    pub fn execute(program: &str, args: &[&str]) -> Result<Output> {
        let mut cmd = Command::new(program);
        cmd.args(args);
        cmd.stdout(Stdio::piped()); // Capture stdout for parsing
        cmd.stderr(Stdio::piped()); // Capture stderr for error messages
        cmd.stdin(Stdio::null());
        cmd.output()
            .with_context(|| format!("Failed to execute command: {}", program))
    }

    /// Check if a command exists using native Rust (which crate)
    pub fn check_command_exists(command: &str) -> bool {
        which::which(command).is_ok()
    }

    pub fn read_file(path: impl AsRef<std::path::Path>) -> Result<String> {
        let path_ref = path.as_ref();
        let path_display = path_ref.display();
        std::fs::read_to_string(path_ref)
            .with_context(|| format!("Failed to read file: {}", path_display))
    }

    /// List directory contents using native Rust
    pub fn list_directory(path: impl AsRef<std::path::Path>) -> Result<Vec<String>> {
        let path_ref = path.as_ref();

        // Check if directory exists first - if not, return empty vector
        if !path_ref.exists() || !path_ref.is_dir() {
            return Ok(Vec::new());
        }

        let mut entries = Vec::new();
        let dir = std::fs::read_dir(path_ref)
            .with_context(|| format!("Failed to read directory: {}", path_ref.display()))?;
        for entry in dir {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            entries.push(name);
        }
        Ok(entries)
    }

    /// Check if a path is a directory using native Rust
    pub fn is_directory(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().is_dir()
    }

    /// Check if a path is a file using native Rust
    pub fn is_file(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().is_file()
    }

    /// Get current user ID using native Rust (Unix only)
    #[cfg(unix)]
    pub fn get_uid() -> Result<u32> {
        use std::os::unix::fs::MetadataExt;
        let metadata = std::fs::metadata(".")?;
        Ok(metadata.uid())
    }

    /// Get current group ID using native Rust (Unix only)
    #[cfg(unix)]
    pub fn get_gid() -> Result<u32> {
        use std::os::unix::fs::MetadataExt;
        let metadata = std::fs::metadata(".")?;
        Ok(metadata.gid())
    }

    /// Check if running on Linux using native Rust
    pub fn is_linux() -> bool {
        cfg!(target_os = "linux")
    }

    /// Copy a file from source to destination using native Rust
    pub fn copy_file(
        from: impl AsRef<std::path::Path>,
        to: impl AsRef<std::path::Path>,
    ) -> Result<u64> {
        let from_ref = from.as_ref();
        let to_ref = to.as_ref();
        std::fs::copy(from_ref, to_ref).with_context(|| {
            format!(
                "Failed to copy file from {} to {}",
                from_ref.display(),
                to_ref.display()
            )
        })
    }

    /// Create a directory and all parent directories using native Rust
    pub fn create_dir_all(path: impl AsRef<std::path::Path>) -> Result<()> {
        let path_ref = path.as_ref();
        std::fs::create_dir_all(path_ref)
            .with_context(|| format!("Failed to create directory: {}", path_ref.display()))
    }

    /// Remove a file using native Rust
    pub fn remove_file(path: impl AsRef<std::path::Path>) -> Result<()> {
        let path_ref = path.as_ref();
        std::fs::remove_file(path_ref)
            .with_context(|| format!("Failed to remove file: {}", path_ref.display()))
    }

    /// Remove a directory and all its contents using native Rust
    pub fn remove_dir_all(path: impl AsRef<std::path::Path>) -> Result<()> {
        let path_ref = path.as_ref();
        std::fs::remove_dir_all(path_ref)
            .with_context(|| format!("Failed to remove directory: {}", path_ref.display()))
    }

    /// Check if a path exists using native Rust
    pub fn path_exists(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().exists()
    }

    /// Set file permissions (Unix only)
    #[cfg(unix)]
    pub fn set_permissions(path: impl AsRef<std::path::Path>, mode: u32) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        let path_ref = path.as_ref();
        std::fs::set_permissions(path_ref, std::fs::Permissions::from_mode(mode))
            .with_context(|| format!("Failed to set permissions for: {}", path_ref.display()))
    }

    /// Get the current user's home directory using native Rust
    pub fn get_home_dir() -> Result<String> {
        std::env::var("HOME")
            .or_else(|_| -> Result<String, std::env::VarError> {
                // Fallback to using whoami crate
                let username = whoami::username();
                if cfg!(target_os = "macos") {
                    Ok(format!("/Users/{}", username))
                } else {
                    Ok(format!("/home/{}", username))
                }
            })
            .with_context(|| "Failed to get home directory")
    }

    /// Execute a shell command (only when absolutely necessary)
    /// Prefer using execute() with specific programs instead
    pub fn execute_shell(command: &str) -> Result<Output> {
        use std::process::Command;
        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdout(Stdio::piped()) // Capture output so it can be parsed
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .output()
            .with_context(|| format!("Failed to execute shell command: {}", command))?;
        Ok(output)
    }
}

/// Trait for executing commands either locally or remotely
pub trait CommandExecutor {
    /// Execute a shell command
    fn execute_shell(&self, command: &str) -> Result<Output>;

    /// Execute a command interactively (with stdin)
    fn execute_interactive(&self, program: &str, args: &[&str]) -> Result<()>;

    /// Check if a command exists
    fn check_command_exists(&self, command: &str) -> Result<bool>;

    /// Check if running on Linux
    fn is_linux(&self) -> Result<bool>;

    /// Read a file
    fn read_file(&self, path: &str) -> Result<String>;

    /// Write a file
    fn write_file(&self, path: &str, content: &[u8]) -> Result<()>;

    /// Create directory recursively
    fn mkdir_p(&self, path: &str) -> Result<()>;

    /// Check if file exists
    fn file_exists(&self, path: &str) -> Result<bool>;

    /// Execute a shell command interactively
    fn execute_shell_interactive(&self, command: &str) -> Result<()>;

    /// Get the current username (for local) or use $USER (for remote)
    fn get_username(&self) -> Result<String>;

    /// List directory contents (native Rust for local, ls command for remote)
    fn list_directory(&self, path: &str) -> Result<Vec<String>>;

    /// Check if path is a directory (native Rust for local, test -d for remote)
    fn is_directory(&self, path: &str) -> Result<bool>;

    /// Get current user ID (native Rust for local, id -u for remote)
    #[cfg(unix)]
    fn get_uid(&self) -> Result<u32>;

    /// Get current group ID (native Rust for local, id -g for remote)
    #[cfg(unix)]
    fn get_gid(&self) -> Result<u32>;

    /// Get the current user's home directory
    fn get_home_dir(&self) -> Result<String>;

    /// Check if this is a local executor
    fn is_local(&self) -> bool;
}

/// Package manager types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
    Apt,
    Yum,
    Dnf,
    Brew,
    Unknown,
}

impl PackageManager {
    /// Detect the package manager available on the system
    pub fn detect(exec: &dyn CommandExecutor) -> Result<Self> {
        if exec.check_command_exists("apt-get")? {
            Ok(PackageManager::Apt)
        } else if exec.check_command_exists("yum")? {
            Ok(PackageManager::Yum)
        } else if exec.check_command_exists("dnf")? {
            Ok(PackageManager::Dnf)
        } else if exec.check_command_exists("brew")? {
            Ok(PackageManager::Brew)
        } else {
            Ok(PackageManager::Unknown)
        }
    }

    /// Install a package using the detected package manager
    pub fn install_package(&self, exec: &dyn CommandExecutor, package: &str) -> Result<()> {
        match self {
            PackageManager::Apt => {
                // Use execute_shell_interactive which handles sudo password injection better
                exec.execute_shell_interactive("sudo apt-get update")?;
                exec.execute_shell_interactive(&format!("sudo apt-get install -y {}", package))?;
            }
            PackageManager::Yum => {
                exec.execute_shell_interactive(&format!("sudo yum install -y {}", package))?;
            }
            PackageManager::Dnf => {
                exec.execute_shell_interactive(&format!("sudo dnf install -y {}", package))?;
            }
            PackageManager::Brew => {
                exec.execute_shell_interactive(&format!("brew install {}", package))?;
            }
            PackageManager::Unknown => {
                anyhow::bail!(
                    "No supported package manager found. Please install {} manually.",
                    package
                );
            }
        }
        Ok(())
    }

    /// Install multiple packages at once
    pub fn install_packages(&self, exec: &dyn CommandExecutor, packages: &[&str]) -> Result<()> {
        match self {
            PackageManager::Apt => {
                exec.execute_interactive("sudo", &["apt-get", "update"])?;
                let mut args = vec!["apt-get", "install", "-y"];
                args.extend(packages.iter().copied());
                exec.execute_interactive("sudo", &args)?;
            }
            PackageManager::Yum => {
                let mut args = vec!["yum", "install", "-y"];
                args.extend(packages.iter().copied());
                exec.execute_interactive("sudo", &args)?;
            }
            PackageManager::Dnf => {
                let mut args = vec!["dnf", "install", "-y"];
                args.extend(packages.iter().copied());
                exec.execute_interactive("sudo", &args)?;
            }
            PackageManager::Brew => {
                let mut args = vec!["brew", "install"];
                args.extend(packages.iter().copied());
                exec.execute_interactive("brew", &args)?;
            }
            PackageManager::Unknown => {
                anyhow::bail!(
                    "No supported package manager found. Please install packages manually."
                );
            }
        }
        Ok(())
    }

    /// Get display name for the package manager
    pub fn display_name(&self) -> &'static str {
        match self {
            PackageManager::Apt => "apt (Debian/Ubuntu)",
            PackageManager::Yum => "yum (RHEL/CentOS)",
            PackageManager::Dnf => "dnf (Fedora)",
            PackageManager::Brew => "brew (macOS)",
            PackageManager::Unknown => "unknown",
        }
    }
}

/// Get username from SSH config file for a given host
/// Returns None if not found (SSH will use defaults)
fn get_ssh_config_username(host: &str) -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let ssh_config_path = PathBuf::from(home).join(".ssh").join("config");

    if !ssh_config_path.exists() {
        return None;
    }

    let content = fs::read_to_string(&ssh_config_path).ok()?;
    let mut in_matching_host = false;
    let mut matched_user: Option<String> = None;

    for line in content.lines() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse Host directive
        if line.starts_with("Host ") || line.starts_with("Host\t") {
            if let Some(host_pattern) = line.split_whitespace().nth(1) {
                // Check if this host pattern matches our target host
                in_matching_host = host_pattern == host
                    || host_pattern == "*"
                    || (host_pattern.contains('*') && simple_wildcard_match(host_pattern, host));
                if in_matching_host {
                    matched_user = None; // Reset user for new host block
                }
            }
        }

        // Parse User directive (only if we're in a matching Host block)
        if in_matching_host {
            if line.starts_with("User ") || line.starts_with("User\t") {
                if let Some(user) = line.split_whitespace().nth(1) {
                    matched_user = Some(user.to_string());
                }
            }
        }
    }

    matched_user
}

/// Simple wildcard matching (supports * at start, end, or both)
fn simple_wildcard_match(pattern: &str, text: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    if pattern.starts_with('*') && pattern.ends_with('*') {
        // *pattern*
        let inner = &pattern[1..pattern.len() - 1];
        text.contains(inner)
    } else if pattern.starts_with('*') {
        // *pattern
        let suffix = &pattern[1..];
        text.ends_with(suffix)
    } else if pattern.ends_with('*') {
        // pattern*
        let prefix = &pattern[..pattern.len() - 1];
        text.starts_with(prefix)
    } else {
        pattern == text
    }
}

/// Prompt user for SSH username if not found in SSH config
fn prompt_ssh_username(host: &str) -> Result<String> {
    let default_user = crate::config::get_default_username();
    print!(
        "SSH username for {} (press Enter for '{}'): ",
        host, default_user
    );
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    let username = input.trim();
    if username.is_empty() {
        Ok(default_user)
    } else {
        Ok(username.to_string())
    }
}

/// Executor that can be either local or remote via SSH
/// Automatically determines execution context based on hostname and config
/// Note: Agent-based execution is handled in halvor-cli where agent module is available
pub enum Executor {
    Local,
    Remote(SshConnection),
}

impl Executor {
    /// Create an executor based on hostname and config
    /// Automatically determines if execution should be local or remote
    pub fn new(hostname: &str, config: &crate::config::EnvConfig) -> Result<Self> {
        // Handle "localhost" as a special case - always local execution
        if hostname == "localhost" || hostname == "127.0.0.1" {
            return Ok(Executor::Local);
        }

        // Check if hostname matches current machine BEFORE requiring it to be in config
        // This allows commands to work on the current machine even if not yet configured
        if let Ok(current_hostname) = crate::utils::hostname::get_current_hostname() {
            let normalized_current = crate::utils::hostname::normalize_hostname(&current_hostname);
            let normalized_input = crate::utils::hostname::normalize_hostname(hostname);

            // Check if hostname matches current machine (exact or normalized)
            // Also check if the base hostname (before first dot) matches
            let current_base = current_hostname
                .split('.')
                .next()
                .unwrap_or(&current_hostname);
            let input_base = hostname.split('.').next().unwrap_or(hostname);

            if hostname.eq_ignore_ascii_case(&current_hostname)
                || hostname.eq_ignore_ascii_case(&normalized_current)
                || normalized_input.eq_ignore_ascii_case(&normalized_current)
                || normalized_input.eq_ignore_ascii_case(&current_hostname)
                || input_base.eq_ignore_ascii_case(current_base)
                || normalized_input.eq_ignore_ascii_case(current_base)
                || normalized_current.eq_ignore_ascii_case(input_base)
            {
                return Ok(Executor::Local);
            }
        }

        // Also check against Tailscale hostname if available
        // This handles cases where the system hostname is "mint.local" but Tailscale hostname is "mint.bombay-pinecone.ts.net"

        // Try to find hostname (with normalization for TLDs)
        let actual_hostname = crate::utils::hostname::find_hostname_in_config(hostname, config)
            .ok_or_else(|| {
                let available_hosts: Vec<String> = config.hosts.keys().cloned().collect();
                anyhow::anyhow!(
                    "Host '{}' not found in config.\n\nAvailable hosts: {}\n\nAdd to .env:\n  HOST_{}_IP=\"<ip-address>\"\n  HOST_{}_HOSTNAME=\"<hostname>\"",
                    hostname,
                    available_hosts.join(", "),
                    hostname.to_uppercase(),
                    hostname.to_uppercase()
                )
            })?;

        // Verify we're using the correct hostname (not a different one)
        if actual_hostname != hostname && !hostname.eq_ignore_ascii_case(&actual_hostname) {
            eprintln!(
                "⚠️  Warning: Hostname '{}' was normalized to '{}'",
                hostname, actual_hostname
            );
        }

        let host_config = config.hosts.get(&actual_hostname).with_context(|| {
            format!(
                "Host '{}' (normalized from '{}') not found in config",
                actual_hostname, hostname
            )
        })?;

        // Get target IP
        let target_ip = if let Some(ip) = &host_config.ip {
            ip.clone()
        } else {
            // If no IP configured, assume remote
            return Ok(Executor::Remote({
                let hostname_val = host_config.hostname.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("No IP or Tailscale hostname configured for {}", hostname)
                })?;
                // Strip trailing dot (DNS absolute notation) which causes SSH resolution issues
                let target_host = hostname_val.trim_end_matches('.').to_string();
                // Get username from SSH config, or prompt user
                let username = get_ssh_config_username(&target_host)
                    .or_else(|| get_ssh_config_username(hostname))
                    .or_else(|| get_ssh_config_username(&actual_hostname))
                    .unwrap_or_else(|| {
                        // No username in SSH config - prompt user
                        prompt_ssh_username(&target_host)
                            .unwrap_or_else(|_| crate::config::get_default_username())
                    });
                let host_with_user = format!("{}@{}", username, target_host);
                // Get sudo password and user from host config
                let sudo_password = host_config.sudo_password.clone();
                let sudo_user = host_config.sudo_user.clone();
                SshConnection::new_with_sudo_password(&host_with_user, sudo_password, sudo_user)?
            }));
        };

        // Get local IP addresses (both regular and Tailscale)
        let local_ips = crate::utils::networking::get_local_ips()?;
        let tailscale_ips = crate::utils::networking::get_tailscale_ips().unwrap_or_default();

        // Check if target IP matches any local IP (regular or Tailscale)
        let is_local_by_ip = local_ips.contains(&target_ip) || tailscale_ips.contains(&target_ip);

        // Also check if the hostname matches the current machine's hostname
        // This is a fallback in case IP comparison fails (e.g., if IPs don't match exactly)
        let is_local_by_hostname = if !is_local_by_ip {
            if let Ok(current_hostname) = crate::utils::hostname::get_current_hostname() {
                if let Some(normalized_current) =
                    crate::utils::hostname::find_hostname_in_config(&current_hostname, config)
                {
                    normalized_current == actual_hostname
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        let is_local = is_local_by_ip || is_local_by_hostname;

        if is_local {
            Ok(Executor::Local)
        } else {
            // Get host configuration for remote connection (try normalized hostname)
            let actual_hostname = crate::utils::hostname::find_hostname_in_config(hostname, config)
                .ok_or_else(|| anyhow::anyhow!("Host '{}' not found in config", hostname))?;
            let host_config = config.hosts.get(&actual_hostname).with_context(|| {
                format!(
                    "Host '{}' not found in .env\n\nAdd configuration to .env:\n  HOST_{}_IP=\"<ip-address>\"\n  HOST_{}_HOSTNAME=\"<hostname>\"",
                    hostname,
                    hostname.to_uppercase(),
                    hostname.to_uppercase()
                )
            })?;

            // Determine which host to connect to (prefer Tailscale hostname, fallback to IP)
            let target_host = if let Some(hostname_val) = &host_config.hostname {
                hostname_val.clone()
            } else if let Some(ip) = &host_config.ip {
                ip.clone()
            } else {
                anyhow::bail!("No IP or Tailscale hostname configured for {}", hostname);
            };

            // Get sudo password and user from host config
            let sudo_password = host_config.sudo_password.clone();
            let sudo_user = host_config.sudo_user.clone();

            // Create SSH connection
            let username = crate::config::get_default_username();
            let host_with_user = format!("{}@{}", username, target_host);
            let conn = SshConnection::new_with_sudo_password(
                &host_with_user,
                sudo_password,
                sudo_user,
            )?;

            Ok(Executor::Remote(conn))
        }
    }

    /// Get the target host (for remote) or hostname (for local)
    pub fn target_host(&self, hostname: &str, config: &crate::config::EnvConfig) -> Result<String> {
        match self {
            Executor::Local => Ok(hostname.to_string()),
            Executor::Remote(_) => {
                let host_config = config
                    .hosts
                    .get(hostname)
                    .with_context(|| format!("Host '{}' not found in config", hostname))?;
                let target_host = if let Some(ip) = &host_config.ip {
                    ip.clone()
                } else if let Some(hostname_val) = &host_config.hostname {
                    hostname_val.clone()
                } else {
                    anyhow::bail!("No IP or hostname configured for {}", hostname);
                };
                Ok(target_host)
            }
        }
    }

    /// Check if this is a local executor
    pub fn is_local(&self) -> bool {
        matches!(self, Executor::Local)
    }
}

impl CommandExecutor for Executor {
    fn execute_shell(&self, command: &str) -> Result<Output> {
        match self {
            Executor::Local => local::execute_shell(command),
            Executor::Remote(exec) => exec.execute_shell(command),
        }
    }

    fn execute_interactive(&self, program: &str, args: &[&str]) -> Result<()> {
        match self {
            Executor::Local => {
                let mut cmd = Command::new(program);
                cmd.args(args);
                cmd.stdin(Stdio::inherit());
                cmd.stdout(Stdio::inherit());
                cmd.stderr(Stdio::inherit());
                let status = cmd.status()?;
                if !status.success() {
                    anyhow::bail!("Command failed: {} {:?}", program, args);
                }
                Ok(())
            }
            Executor::Remote(exec) => exec.execute_interactive(program, args),
        }
    }

    fn check_command_exists(&self, command: &str) -> Result<bool> {
        match self {
            Executor::Local => Ok(local::check_command_exists(command)),
            Executor::Remote(exec) => exec.check_command_exists(command),
        }
    }

    fn is_linux(&self) -> Result<bool> {
        match self {
            Executor::Local => Ok(local::is_linux()),
            Executor::Remote(exec) => exec.is_linux(),
        }
    }

    fn read_file(&self, path: &str) -> Result<String> {
        match self {
            Executor::Local => local::read_file(path),
            Executor::Remote(exec) => exec.read_file(path),
        }
    }

    fn write_file(&self, path: &str, content: &[u8]) -> Result<()> {
        match self {
            Executor::Local => {
                // Check if path requires sudo (system directories)
                let needs_sudo = path.starts_with("/etc/")
                    || path.starts_with("/usr/local/bin/")
                    || path.starts_with("/opt/")
                    || path.starts_with("/var/lib/");

                if needs_sudo {
                    // Use sudo tee for system paths
                    use std::process::{Command, Stdio};
                    let mut cmd = Command::new("sudo");
                    cmd.arg("tee");
                    cmd.arg(path);
                    cmd.stdin(Stdio::piped());
                    cmd.stdout(Stdio::null());
                    cmd.stderr(Stdio::inherit());

                    let mut child = cmd.spawn().with_context(|| {
                        format!("Failed to spawn sudo command for writing file")
                    })?;

                    if let Some(mut stdin) = child.stdin.take() {
                        stdin.write_all(content)?;
                        stdin.flush()?;
                    }

                    let status = child
                        .wait()
                        .with_context(|| format!("Failed to write file: {}", path))?;

                    if !status.success() {
                        anyhow::bail!("Failed to write file: {}", path);
                    }
                } else {
                    std::fs::write(path, content)
                        .with_context(|| format!("Failed to write file: {}", path))?;
                }
                Ok(())
            }
            Executor::Remote(exec) => exec.write_file(path, content),
        }
    }

    fn mkdir_p(&self, path: &str) -> Result<()> {
        match self {
            Executor::Local => {
                std::fs::create_dir_all(path)
                    .with_context(|| format!("Failed to create directory: {}", path))?;
                Ok(())
            }
            Executor::Remote(exec) => exec.mkdir_p(path),
        }
    }

    fn file_exists(&self, path: &str) -> Result<bool> {
        match self {
            Executor::Local => Ok(local::is_file(path)),
            Executor::Remote(exec) => exec.file_exists(path),
        }
    }

    fn execute_shell_interactive(&self, command: &str) -> Result<()> {
        match self {
            Executor::Local => {
                let mut cmd = Command::new("sh");
                cmd.arg("-c");
                cmd.arg(command);
                cmd.stdin(Stdio::inherit());
                cmd.stdout(Stdio::inherit());
                cmd.stderr(Stdio::inherit());
                // Set environment variables to disable pagers
                cmd.env("PAGER", "cat");
                cmd.env("SYSTEMD_PAGER", "cat");
                cmd.env("DEBIAN_FRONTEND", "noninteractive");
                let status = cmd.status()?;
                if !status.success() {
                    anyhow::bail!("Shell command failed");
                }
                Ok(())
            }
            Executor::Remote(exec) => exec.execute_shell_interactive(command),
        }
    }

    fn get_username(&self) -> Result<String> {
        match self {
            Executor::Local => Ok(whoami::username()),
            Executor::Remote(exec) => exec.get_username(),
        }
    }

    fn list_directory(&self, path: &str) -> Result<Vec<String>> {
        match self {
            Executor::Local => local::list_directory(path),
            Executor::Remote(exec) => exec.list_directory(path),
        }
    }

    fn is_directory(&self, path: &str) -> Result<bool> {
        match self {
            Executor::Local => Ok(local::is_directory(path)),
            Executor::Remote(exec) => exec.is_directory(path),
        }
    }

    #[cfg(unix)]
    fn get_uid(&self) -> Result<u32> {
        match self {
            Executor::Local => local::get_uid(),
            Executor::Remote(exec) => exec.get_uid(),
        }
    }

    #[cfg(unix)]
    fn get_gid(&self) -> Result<u32> {
        match self {
            Executor::Local => local::get_gid(),
            Executor::Remote(exec) => exec.get_gid(),
        }
    }

    fn get_home_dir(&self) -> Result<String> {
        match self {
            Executor::Local => local::get_home_dir(),
            Executor::Remote(exec) => exec.get_home_dir(),
        }
    }

    fn is_local(&self) -> bool {
        self.is_local()
    }
}

/// Remote command executor (SSH) - SshConnection already implements CommandExecutor
impl CommandExecutor for SshConnection {
    fn execute_shell(&self, command: &str) -> Result<Output> {
        self.execute_shell(command)
    }

    fn execute_interactive(&self, program: &str, args: &[&str]) -> Result<()> {
        self.execute_interactive(program, args)
    }

    fn check_command_exists(&self, command: &str) -> Result<bool> {
        self.check_command_exists(command)
    }

    fn is_linux(&self) -> Result<bool> {
        self.is_linux()
    }

    fn read_file(&self, path: &str) -> Result<String> {
        self.read_file(path)
    }

    fn write_file(&self, path: &str, content: &[u8]) -> Result<()> {
        self.write_file(path, content)
    }

    fn mkdir_p(&self, path: &str) -> Result<()> {
        self.mkdir_p(path)
    }

    fn file_exists(&self, path: &str) -> Result<bool> {
        self.file_exists(path)
    }

    fn execute_shell_interactive(&self, command: &str) -> Result<()> {
        self.execute_shell_interactive(command)
    }

    fn get_username(&self) -> Result<String> {
        let output = self.execute_shell("whoami")?;
        let username = String::from_utf8(output.stdout)?.trim().to_string();
        Ok(username)
    }

    fn list_directory(&self, path: &str) -> Result<Vec<String>> {
        SshConnection::list_directory(self, path)
    }

    fn is_directory(&self, path: &str) -> Result<bool> {
        SshConnection::is_directory(self, path)
    }

    #[cfg(unix)]
    fn get_uid(&self) -> Result<u32> {
        SshConnection::get_uid(self)
    }

    #[cfg(unix)]
    fn get_gid(&self) -> Result<u32> {
        SshConnection::get_gid(self)
    }

    fn get_home_dir(&self) -> Result<String> {
        let output = self.execute_shell("echo $HOME")?;
        let home_dir = String::from_utf8(output.stdout)?.trim().to_string();
        Ok(home_dir)
    }

    fn is_local(&self) -> bool {
        false
    }
}
