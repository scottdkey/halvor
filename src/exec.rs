use anyhow::{Context, Result};
use std::io::Write;
use std::process::{Command, Output, Stdio};

/// SSH connection for remote command execution
pub struct SshConnection {
    host: String,
    use_key_auth: bool,
}

impl SshConnection {
    pub fn new(host: &str) -> Result<Self> {
        // Test if key-based auth works
        let test_output = Command::new("ssh")
            .args([
                "-o",
                "ConnectTimeout=1",
                "-o",
                "BatchMode=yes",
                "-o",
                "PreferredAuthentications=publickey",
                "-o",
                "PasswordAuthentication=no",
                "-o",
                "StrictHostKeyChecking=no",
                host,
                "echo",
                "test",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output();

        let use_key_auth = test_output.is_ok() && test_output.unwrap().status.success();

        Ok(Self {
            host: host.to_string(),
            use_key_auth,
        })
    }

    fn build_ssh_args(&self) -> Vec<String> {
        let mut args = vec!["-o".to_string(), "StrictHostKeyChecking=no".to_string()];

        if self.use_key_auth {
            args.extend([
                "-o".to_string(),
                "PreferredAuthentications=publickey".to_string(),
                "-o".to_string(),
                "PasswordAuthentication=no".to_string(),
            ]);
        } else {
            args.extend([
                "-o".to_string(),
                "PreferredAuthentications=publickey,keyboard-interactive,password".to_string(),
            ]);
        }

        args.push(self.host.clone());
        args
    }

    pub fn execute_simple(&self, program: &str, args: &[&str]) -> Result<Output> {
        let mut ssh_args = self.build_ssh_args();

        // Execute command directly without shell
        ssh_args.push(program.to_string());
        for arg in args {
            ssh_args.push(arg.to_string());
        }

        let output = Command::new("ssh")
            .args(&ssh_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .output()
            .with_context(|| format!("Failed to execute command: {}", program))?;

        Ok(output)
    }

    pub fn execute_shell(&self, command: &str) -> Result<Output> {
        let mut ssh_args = self.build_ssh_args();
        ssh_args.push("sh".to_string());
        ssh_args.push("-c".to_string());
        ssh_args.push(command.to_string());

        let output = Command::new("ssh")
            .args(&ssh_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .output()
            .with_context(|| format!("Failed to execute shell command"))?;

        Ok(output)
    }

    pub fn execute_interactive(&self, program: &str, args: &[&str]) -> Result<()> {
        let mut ssh_args = self.build_ssh_args();
        ssh_args.push("-tt".to_string()); // Force TTY for interactive

        // Execute command directly
        ssh_args.push(program.to_string());
        for arg in args {
            ssh_args.push(arg.to_string());
        }

        let status = Command::new("ssh")
            .args(&ssh_args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .with_context(|| format!("Failed to execute interactive command: {}", program))?;

        if !status.success() {
            anyhow::bail!(
                "Command '{}' failed with exit code: {}",
                program,
                status.code().unwrap_or(1)
            );
        }

        Ok(())
    }

    pub fn execute_shell_interactive(&self, command: &str) -> Result<()> {
        let mut ssh_args = self.build_ssh_args();
        ssh_args.push("-tt".to_string()); // Force TTY for interactive
        ssh_args.push("sh".to_string());
        ssh_args.push("-c".to_string());
        ssh_args.push(command.to_string());

        let status = Command::new("ssh")
            .args(&ssh_args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .with_context(|| format!("Failed to execute interactive shell command"))?;

        if !status.success() {
            anyhow::bail!(
                "Shell command failed with exit code: {}",
                status.code().unwrap_or(1)
            );
        }

        Ok(())
    }

    pub fn check_command_exists(&self, command: &str) -> Result<bool> {
        let output = self.execute_simple("command", &["-v", command])?;
        Ok(output.status.success())
    }

    pub fn is_linux(&self) -> Result<bool> {
        let output = self.execute_simple("uname", &[])?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.trim() != "Darwin")
    }

    pub fn read_file(&self, path: &str) -> Result<String> {
        let output = self.execute_simple("cat", &[path])?;
        if !output.status.success() {
            anyhow::bail!("Failed to read file: {}", path);
        }
        String::from_utf8(output.stdout)
            .with_context(|| format!("Failed to decode file contents: {}", path))
    }

    pub fn write_file(&self, path: &str, content: &[u8]) -> Result<()> {
        let mut ssh_args = self.build_ssh_args();
        ssh_args.push("sh".to_string());
        ssh_args.push("-c".to_string());
        ssh_args.push(format!("cat > {}", shell_escape(path)));

        let mut cmd = Command::new("ssh");
        cmd.args(&ssh_args);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::inherit());

        let mut child = cmd
            .spawn()
            .with_context(|| format!("Failed to spawn SSH command for writing file"))?;

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

        Ok(())
    }

    pub fn mkdir_p(&self, path: &str) -> Result<()> {
        let output = self.execute_simple("mkdir", &["-p", path])?;
        if !output.status.success() {
            anyhow::bail!("Failed to create directory: {}", path);
        }
        Ok(())
    }

    pub fn file_exists(&self, path: &str) -> Result<bool> {
        let output = self.execute_simple("test", &["-f", path])?;
        Ok(output.status.success())
    }
}

/// Escape a string for safe use in shell commands
fn shell_escape(s: &str) -> String {
    // Simple escaping - wrap in single quotes and escape single quotes
    if s.is_empty() {
        return "''".to_string();
    }

    // If string contains no special characters, return as-is
    if s.chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '/' || c == '.' || c == '$')
    {
        return s.to_string();
    }

    // Escape single quotes by ending quote, adding escaped quote, starting new quote
    let escaped = s.replace('\'', "'\"'\"'");
    format!("'{}'", escaped)
}

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

    pub fn check_command_exists(command: &str) -> bool {
        Command::new("command")
            .arg("-v")
            .arg(command)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    pub fn read_file(path: impl AsRef<std::path::Path>) -> Result<String> {
        let path_ref = path.as_ref();
        let path_display = path_ref.display();
        std::fs::read_to_string(path_ref)
            .with_context(|| format!("Failed to read file: {}", path_display))
    }

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
            Executor::Local => {
                #[cfg(target_os = "linux")]
                return Ok(true);
                #[cfg(not(target_os = "linux"))]
                return Ok(false);
            }
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
            Executor::Local => Ok(std::path::Path::new(path).exists()),
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
}
