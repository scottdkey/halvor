use anyhow::{Context, Result};
use std::process::{Command, Output, Stdio};

// Import SshConnection from ssh module
use crate::ssh::SshConnection;

/// Local command execution helpers
pub mod local {
    use super::*;

    pub fn execute(program: &str, args: &[&str]) -> Result<Output> {
        let mut cmd = Command::new(program);
        cmd.args(args);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
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

    /// Execute a shell command (only when absolutely necessary)
    /// Prefer using execute() with specific programs instead
    pub fn execute_shell(command: &str) -> Result<Output> {
        use std::process::Command;
        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .output()
            .with_context(|| format!("Failed to execute shell command: {}", command))?;
        Ok(output)
    }
}

/// Trait for executing commands either locally or remotely
pub trait CommandExecutor {
    /// Execute a simple command
    fn execute_simple(&self, program: &str, args: &[&str]) -> Result<Output>;

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
    pub fn detect<E: CommandExecutor>(exec: &E) -> Result<Self> {
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
    pub fn install_package<E: CommandExecutor>(&self, exec: &E, package: &str) -> Result<()> {
        match self {
            PackageManager::Apt => {
                exec.execute_interactive("sudo", &["apt-get", "update"])?;
                exec.execute_interactive("sudo", &["apt-get", "install", "-y", package])?;
            }
            PackageManager::Yum => {
                exec.execute_interactive("sudo", &["yum", "install", "-y", package])?;
            }
            PackageManager::Dnf => {
                exec.execute_interactive("sudo", &["dnf", "install", "-y", package])?;
            }
            PackageManager::Brew => {
                exec.execute_interactive("brew", &["install", package])?;
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
    pub fn _install_packages<E: CommandExecutor>(&self, exec: &E, packages: &[&str]) -> Result<()> {
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

/// Executor that can be either local or remote (SSH)
/// Automatically determines execution context based on hostname and config
pub enum Executor {
    Local,
    Remote(SshConnection),
}

impl Executor {
    /// Create an executor based on hostname and config
    /// Automatically determines if execution should be local or remote
    pub fn new(hostname: &str, config: &crate::config::EnvConfig) -> Result<Self> {
        let host_config = config
            .hosts
            .get(hostname)
            .with_context(|| format!("Host '{}' not found in config", hostname))?;

        // Get target IP
        let target_ip = if let Some(ip) = &host_config.ip {
            ip.clone()
        } else {
            // If no IP configured, assume remote
            return Ok(Executor::Remote({
                let target_host = host_config.tailscale.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("No IP or Tailscale hostname configured for {}", hostname)
                })?;
                let default_user = crate::config::get_default_username();
                let host_with_user = format!("{}@{}", default_user, target_host);
                SshConnection::new(&host_with_user)?
            }));
        };

        // Get local IP addresses
        let local_ips = crate::networking::get_local_ips()?;

        // Check if target IP matches any local IP
        let is_local = local_ips.contains(&target_ip);

        if is_local {
            Ok(Executor::Local)
        } else {
            // Get host configuration for remote connection
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

            // Create SSH connection
            let default_user = crate::config::get_default_username();
            let host_with_user = format!("{}@{}", default_user, target_host);
            let ssh_conn = SshConnection::new(&host_with_user)?;

            Ok(Executor::Remote(ssh_conn))
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
                } else if let Some(tailscale) = &host_config.tailscale {
                    tailscale.clone()
                } else {
                    anyhow::bail!("No IP or Tailscale hostname configured for {}", hostname);
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
    fn execute_simple(&self, program: &str, args: &[&str]) -> Result<Output> {
        match self {
            Executor::Local => local::execute(program, args),
            Executor::Remote(exec) => exec.execute_simple(program, args),
        }
    }

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
                std::fs::write(path, content)
                    .with_context(|| format!("Failed to write file: {}", path))?;
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
}

/// Remote command executor (SSH) - SshConnection already implements CommandExecutor
impl CommandExecutor for SshConnection {
    fn execute_simple(&self, program: &str, args: &[&str]) -> Result<Output> {
        self.execute_simple(program, args)
    }

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
        let output = self.execute_simple("whoami", &[])?;
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
}
