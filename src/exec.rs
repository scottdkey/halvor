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

    pub fn execute_status(program: &str, args: &[&str]) -> Result<std::process::ExitStatus> {
        let mut cmd = Command::new(program);
        cmd.args(args);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());
        Ok(cmd.status()?)
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
}
