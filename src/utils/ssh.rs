use crate::config::{self, EnvConfig};
use crate::utils::exec::local;
use anyhow::{Context, Result};
use std::io::{self, Write};
use std::process::{Command, Output, Stdio};

/// SSH connection for remote command execution
pub struct SshConnection {
    pub(crate) host: String,
    pub(crate) use_key_auth: bool,
    pub(crate) sudo_password: Option<String>,
    pub(crate) sudo_user: Option<String>, // Sudo user from SUDO_USER env var
}

impl SshConnection {
    pub fn new(host: &str) -> Result<Self> {
        // Test if key-based auth works (with longer timeout for initial connection)
        let test_output = Command::new("ssh")
            .args([
                "-o",
                "ConnectTimeout=10", // Increased from 1 to 10 seconds
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
            sudo_password: None,
            sudo_user: None,
        })
    }

    /// Create SSH connection with sudo password and user
    pub fn new_with_sudo_password(
        host: &str,
        sudo_password: Option<String>,
        sudo_user: Option<String>,
    ) -> Result<Self> {
        // Test if key-based auth works (with longer timeout for initial connection)
        let test_output = Command::new("ssh")
            .args([
                "-o",
                "ConnectTimeout=10", // Increased from 1 to 10 seconds
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
            sudo_password,
            sudo_user,
        })
    }

    fn build_ssh_args(&self) -> Vec<String> {
        let mut args = vec![
            "-o".to_string(),
            "StrictHostKeyChecking=no".to_string(),
            "-o".to_string(),
            "ConnectTimeout=30".to_string(), // 30 second timeout for initial connections
        ];

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
        // If command contains sudo and we have a password, inject it
        let final_command = if command.contains("sudo ") && self.sudo_password.is_some() {
            let password = self.sudo_password.as_ref().unwrap();
            // Use echo with password and newline piped to sudo for reliable password passing
            // Structure: echo 'password' | sudo -S command
            // Simple and reliable - echo automatically adds newline which sudo needs
            let escaped_password = shell_escape(password);
            let sudo_prefix = if let Some(ref sudo_user) = self.sudo_user {
                format!(
                    "echo {} | sudo -S -u {} ",
                    escaped_password,
                    shell_escape(sudo_user)
                )
            } else {
                format!("echo {} | sudo -S ", escaped_password)
            };
            command.replace("sudo ", &sudo_prefix)
        } else {
            command.to_string()
        };

        let mut ssh_args = self.build_ssh_args();
        // For non-interactive commands, add BatchMode=yes if key auth works to prevent hanging
        // If key auth doesn't work, we can't use BatchMode (needs password), so it will hang
        // In that case, the caller should use execute_shell_interactive instead
        if self.use_key_auth {
            ssh_args.insert(ssh_args.len() - 1, "-o".to_string());
            ssh_args.insert(ssh_args.len() - 1, "BatchMode=yes".to_string());
        }
        ssh_args.push("sh".to_string());
        ssh_args.push("-c".to_string());
        ssh_args.push(final_command);

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
        // If this is a sudo command and we have a password, use it
        if program == "sudo" && self.sudo_password.is_some() {
            return self.execute_sudo_with_password(args);
        }

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

    /// Execute sudo command with password when available
    fn execute_sudo_with_password(&self, args: &[&str]) -> Result<()> {
        let password = self.sudo_password.as_ref().unwrap();
        // Escape the password for shell
        let escaped_password = shell_escape(password);

        // Build the sudo command with password via stdin
        // Format: echo 'password' | sudo -S [-u user] command args...
        let mut sudo_cmd = format!("echo {} | sudo -S", escaped_password);

        // Add -u user if SUDO_USER is set
        if let Some(ref sudo_user) = self.sudo_user {
            sudo_cmd.push_str(&format!(" -u {}", shell_escape(sudo_user)));
        }

        for arg in args {
            sudo_cmd.push(' ');
            sudo_cmd.push_str(&shell_escape(arg));
        }

        let mut ssh_args = self.build_ssh_args();
        ssh_args.push("-tt".to_string()); // Force TTY for sudo
        ssh_args.push("sh".to_string());
        ssh_args.push("-c".to_string());
        ssh_args.push(sudo_cmd);

        let status = Command::new("ssh")
            .args(&ssh_args)
            .stdin(Stdio::null()) // Password is piped via echo, so no stdin needed
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .with_context(|| "Failed to execute sudo command with password")?;

        if !status.success() {
            anyhow::bail!(
                "Sudo command failed with exit code: {}",
                status.code().unwrap_or(1)
            );
        }

        Ok(())
    }

    pub fn execute_shell_interactive(&self, command: &str) -> Result<()> {
        // If command contains sudo and we have a password, inject it
        let final_command = if command.contains("sudo ") && self.sudo_password.is_some() {
            let password = self.sudo_password.as_ref().unwrap();
            // Use echo with password and newline piped to sudo for reliable password passing
            // Structure: echo 'password' | sudo -S command
            // Simple and reliable - echo automatically adds newline which sudo needs
            let escaped_password = shell_escape(password);
            let sudo_prefix = if let Some(ref sudo_user) = self.sudo_user {
                format!(
                    "echo {} | sudo -S -u {} ",
                    escaped_password,
                    shell_escape(sudo_user)
                )
            } else {
                format!("echo {} | sudo -S ", escaped_password)
            };
            let replaced = command.replace("sudo ", &sudo_prefix);
            // Debug: verify command was replaced (but don't print password)
            if replaced != command {
                println!("  [DEBUG] Sudo password injection: command modified");
            }
            replaced
        } else if command.contains("sudo ") {
            // Sudo command but no password - warn user and run as-is (will prompt)
            eprintln!("⚠️  Warning: Sudo command detected but no password available in config.");
            eprintln!("   Command will prompt for password interactively.");
            eprintln!("   To avoid prompts, set HOST_<name>_SUDO_PASS in your 1Password vault.");
            command.to_string()
        } else {
            command.to_string()
        };

        let mut ssh_args = self.build_ssh_args();
        ssh_args.push("-tt".to_string()); // Force TTY for interactive
        ssh_args.push("sh".to_string());
        ssh_args.push("-c".to_string());
        ssh_args.push(final_command);

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
        // For remote, we still need to check via command
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

    pub fn list_directory(&self, path: &str) -> Result<Vec<String>> {
        let output = self.execute_simple("ls", &["-1", path])?;
        if !output.status.success() {
            return Ok(Vec::new());
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|s| s.trim().to_string())
            .collect())
    }

    pub fn is_directory(&self, path: &str) -> Result<bool> {
        let output = self.execute_simple("test", &["-d", path])?;
        Ok(output.status.success())
    }

    #[cfg(unix)]
    pub fn get_uid(&self) -> Result<u32> {
        let output = self.execute_simple("id", &["-u"])?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout
            .trim()
            .parse::<u32>()
            .with_context(|| format!("Failed to parse UID: {}", stdout))
    }

    #[cfg(unix)]
    pub fn get_gid(&self) -> Result<u32> {
        let output = self.execute_simple("id", &["-g"])?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout
            .trim()
            .parse::<u32>()
            .with_context(|| format!("Failed to parse GID: {}", stdout))
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

fn _remove_ssh_host_key(host: &str) -> Result<()> {
    println!("Removing host key for {} from known_hosts...", host);

    // Use exec::local for local command execution
    let output = local::execute("ssh-keygen", &["-R", host])?;

    if output.status.success() {
        println!("✓ Removed host key for {}", host);
        Ok(())
    } else {
        anyhow::bail!("Failed to remove host key for {}", host);
    }
}

fn _prompt_remove_host_key(host: &str) -> Result<bool> {
    print!("Remove host key for {} from known_hosts? [y/N]: ", host);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let response = input.trim().to_lowercase();
    Ok(response == "y" || response == "yes")
}

fn _connect_ssh_key_based(host: &str, user: Option<&str>, ssh_args: &[String]) -> Result<()> {
    // First, test if key-based auth works using SshConnection
    let host_str = if let Some(u) = user {
        format!("{}@{}", u, host)
    } else {
        host.to_string()
    };

    // Use SshConnection to test key-based auth
    let ssh_conn = SshConnection::new(&host_str)?;
    if !ssh_conn.use_key_auth {
        anyhow::bail!("Key-based authentication not available");
    }

    // Key-based auth works, now actually connect
    let mut cmd = Command::new("ssh");

    // Use key-based authentication only (no password prompts)
    cmd.args([
        "-o",
        "PreferredAuthentications=publickey",
        "-o",
        "PasswordAuthentication=no",
        "-o",
        "StrictHostKeyChecking=no",
    ]);

    cmd.arg(&host_str);

    if !ssh_args.is_empty() {
        cmd.args(ssh_args);
    }

    // Allow interactive output for the actual connection
    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    // Execute SSH - this will open the interactive session
    let status = cmd.status()?;

    // If SSH exits successfully, we're done
    if status.success() {
        std::process::exit(0);
    } else {
        // SSH connection failed, return error so we can try password-based auth
        anyhow::bail!(
            "SSH connection failed with exit code: {}",
            status.code().unwrap_or(1)
        );
    }
}

fn _connect_ssh(host: &str, user: Option<&str>, ssh_args: &[String]) -> Result<()> {
    let mut cmd = Command::new("ssh");

    // Add options to allow password authentication (fallback)
    cmd.args([
        "-o",
        "PreferredAuthentications=keyboard-interactive,password,publickey",
        "-o",
        "StrictHostKeyChecking=no",
    ]);

    // Build host string with optional user
    let host_str = if let Some(u) = user {
        format!("{}@{}", u, host)
    } else {
        host.to_string()
    };

    cmd.arg(&host_str);

    if !ssh_args.is_empty() {
        cmd.args(ssh_args);
    }

    // Allow interactive authentication (password prompts, etc.)
    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    // Execute SSH - this will block and allow password prompts
    let status = cmd.status()?;

    // If SSH exits successfully, we're done
    if status.success() {
        std::process::exit(0);
    } else {
        // SSH connection failed, return error so we can try next host
        anyhow::bail!(
            "SSH connection failed with exit code: {}",
            status.code().unwrap_or(1)
        );
    }
}

/// Copy SSH key to remote host
///
/// This function handles copying SSH keys to a remote host, prompting for passwords as needed.
/// It uses ssh-copy-id when the server and target users are the same, or manually installs
/// the key when they differ (requiring sudo).
///
/// # Arguments
/// * `host` - The hostname or IP address of the remote host
/// * `server_user` - Optional username to SSH into the server (defaults to local username)
/// * `target_user` - Optional username where the key should be installed (defaults to server_user)
pub fn copy_ssh_key(
    host: &str,
    server_user: Option<&str>,
    target_user: Option<&str>,
) -> Result<()> {
    // Determine server username (to SSH into the server)
    let default_server_user = config::get_default_username();
    let server_username = if let Some(u) = server_user {
        u.to_string()
    } else {
        print!(
            "Server username to SSH into {} (press Enter for '{}'): ",
            host, default_server_user
        );
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let input_username = input.trim();
        if input_username.is_empty() {
            default_server_user
        } else {
            input_username.to_string()
        }
    };

    // Determine target username (where to install the key)
    let target_username = if let Some(u) = target_user {
        u.to_string()
    } else {
        // Default to the same as server username, but allow override
        let default_target = server_username.clone();
        print!(
            "Target username to install key for (press Enter for '{}'): ",
            default_target
        );
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let input_username = input.trim();
        if input_username.is_empty() {
            default_target
        } else {
            input_username.to_string()
        }
    };

    println!(
        "Copying SSH public key to {}@{} (installing for user: {})...",
        server_username, host, target_username
    );

    // Find the public key file using config module
    let home = crate::config::config_manager::get_home_dir()?;
    let home_str = home.to_string_lossy().to_string();

    let pubkey_paths = [
        format!("{}/.ssh/id_rsa.pub", home_str),
        format!("{}/.ssh/id_ed25519.pub", home_str),
        format!("{}/.ssh/id_ecdsa.pub", home_str),
    ];

    let pubkey_content = pubkey_paths
        .iter()
        .find_map(|path| std::fs::read_to_string(path).ok())
        .ok_or_else(|| {
            anyhow::anyhow!("No SSH public key found. Please generate one with: ssh-keygen")
        })?;

    let pubkey_line = pubkey_content.trim();

    // Build host string with server username
    let host_str = format!("{}@{}", server_username, host);

    // If target user is different from server user, we need to use sudo
    if target_username != server_username {
        // First, check if the user exists
        let check_user_cmd = format!(r#"id -u {} >/dev/null 2>&1"#, target_username);

        println!(
            "Checking if user '{}' exists on remote system...",
            target_username
        );
        // Use SshConnection for non-interactive command execution
        let ssh_conn = SshConnection::new(&host_str)?;
        let user_exists = ssh_conn.execute_shell(&check_user_cmd)?.status.success();

        if !user_exists {
            // User doesn't exist, create it and set a password
            println!(
                "User '{}' does not exist. Creating user...",
                target_username
            );

            // Prompt for password
            print!("Set password for user '{}' (required): ", target_username);
            io::stdout().flush()?;

            let mut password_input = String::new();
            io::stdin().read_line(&mut password_input)?;
            let password = password_input.trim();

            if password.is_empty() {
                anyhow::bail!("Password is required to create user '{}'", target_username);
            }

            // Create user and set password using chpasswd (more secure than passing password in command)
            // We'll use a here-document approach via SSH
            let create_user_cmd = format!(
                r#"sudo useradd -m -s /bin/bash {} && echo '{}:{}' | sudo chpasswd"#,
                target_username, target_username, password
            );

            // Use SshConnection for interactive command execution (needs TTY for sudo)
            let ssh_conn = SshConnection::new(&host_str)?;
            ssh_conn
                .execute_shell_interactive(&create_user_cmd)
                .with_context(|| {
                    format!("Failed to create user {} on {}", target_username, host)
                })?;

            println!("✓ User '{}' created with password", target_username);
        } else {
            // User exists, check if password is set
            println!("✓ User '{}' already exists", target_username);

            // Check if password is set by reading /etc/shadow and parsing with Rust
            let ssh_conn = SshConnection::new(&host_str)?;
            let shadow_content = ssh_conn.execute_shell("sudo cat /etc/shadow")?;
            let shadow_text = String::from_utf8_lossy(&shadow_content.stdout);

            // Parse shadow file: find line starting with username, extract password field (2nd field)
            let password_status = shadow_text
                .lines()
                .find(|line| line.starts_with(&format!("{}:", target_username)))
                .and_then(|line| {
                    line.split(':').nth(1) // Get password field (2nd field, index 1)
                })
                .map(|pwd_field| {
                    // Empty password field means ! or * or empty string
                    if pwd_field.is_empty() || pwd_field == "!" || pwd_field == "*" {
                        "NO_PASSWORD"
                    } else {
                        "HAS_PASSWORD"
                    }
                })
                .unwrap_or("HAS_PASSWORD"); // Default to HAS_PASSWORD if user not found

            // Check password status (already determined from parsing)
            if password_status == "NO_PASSWORD" {
                println!("User '{}' exists but has no password set.", target_username);
                print!(
                    "Set password for user '{}' (press Enter to skip): ",
                    target_username
                );
                io::stdout().flush()?;

                let mut password_input = String::new();
                io::stdin().read_line(&mut password_input)?;
                let password = password_input.trim();

                if !password.is_empty() {
                    // Set the password
                    let set_password_cmd =
                        format!(r#"echo '{}:{}' | sudo chpasswd"#, target_username, password);

                    // Use SshConnection for interactive command execution (needs TTY for sudo)
                    let ssh_conn = SshConnection::new(&host_str)?;
                    if ssh_conn
                        .execute_shell_interactive(&set_password_cmd)
                        .is_ok()
                    {
                        println!("✓ Password set for user '{}'", target_username);
                    } else {
                        eprintln!(
                            "Warning: Failed to set password for user '{}'",
                            target_username
                        );
                    }
                } else {
                    println!("Skipping password setup - user can login with SSH keys only");
                }
            } else {
                println!("✓ User '{}' has a password set", target_username);
            }
        }

        // Now install the SSH key
        // Use getent to get the actual home directory path
        let append_cmd = format!(
            r#"HOME_DIR=$(getent passwd {} | cut -d: -f6) && sudo mkdir -p "$HOME_DIR/.ssh" && sudo chmod 700 "$HOME_DIR/.ssh" && echo '{}' | sudo tee -a "$HOME_DIR/.ssh/authorized_keys" > /dev/null && sudo chown {}:{} "$HOME_DIR/.ssh/authorized_keys" && sudo chmod 600 "$HOME_DIR/.ssh/authorized_keys" && sudo chown {}:{} "$HOME_DIR/.ssh""#,
            target_username,
            pubkey_line,
            target_username,
            target_username,
            target_username,
            target_username
        );

        println!("Installing SSH key for user '{}'...", target_username);
        // Use SshConnection for interactive command execution (needs TTY for sudo)
        let ssh_conn = SshConnection::new(&host_str)?;
        ssh_conn
            .execute_shell_interactive(&append_cmd)
            .with_context(|| {
                format!(
                    "Failed to install SSH key for user {} on {}",
                    target_username, host
                )
            })?;

        println!(
            "✓ SSH key copied successfully to {}@{} (installed for user: {})",
            server_username, host, target_username
        );
        Ok(())
    } else {
        // Same user - manually install the key using our SSH infrastructure
        // This avoids the ssh-copy-id TTY issues and uses our proven interactive SSH code
        println!("Setting up SSH key authentication...");
        io::stdout().flush()?;

        // Check if key is already installed to avoid duplicates
        // Read authorized_keys file and check if key exists using native Rust
        let ssh_conn = SshConnection::new(&host_str)?;

        // Get the target user's home directory using getent (more reliable than $HOME)
        let get_home_cmd = format!(
            r#"getent passwd {} | cut -d: -f6"#,
            shell_escape(&target_username)
        );
        let home_dir_output = ssh_conn.execute_shell(&get_home_cmd).with_context(|| {
            format!("Failed to get home directory for user {}", target_username)
        })?;

        if !home_dir_output.status.success() {
            anyhow::bail!(
                "Failed to get home directory for user {}. User may not exist.",
                target_username
            );
        }

        let home_dir = String::from_utf8_lossy(&home_dir_output.stdout)
            .trim()
            .to_string();

        if home_dir.is_empty() {
            anyhow::bail!(
                "Home directory for user {} is empty or could not be determined",
                target_username
            );
        }

        let ssh_dir = format!("{}/.ssh", home_dir);
        let authorized_keys_path = format!("{}/authorized_keys", ssh_dir);

        // Check if key is already installed
        let key_status =
            if let Ok(authorized_keys_content) = ssh_conn.read_file(&authorized_keys_path) {
                // Check if the public key line is already in the file
                if authorized_keys_content
                    .lines()
                    .any(|line| line.trim() == pubkey_line.trim())
                {
                    "EXISTS"
                } else {
                    "NOT_FOUND"
                }
            } else {
                "NOT_FOUND"
            };

        if key_status == "EXISTS" {
            println!("✓ SSH key already installed on {}", host_str);
            return Ok(());
        }

        // Install the key using interactive SSH (will prompt for password)
        println!("Installing SSH key...");

        // First, ensure .ssh directory exists and has correct permissions
        // Use the actual home directory path we got from getent
        let setup_dir_cmd = format!(
            r#"mkdir -p {} && chmod 700 {}"#,
            shell_escape(&ssh_dir),
            shell_escape(&ssh_dir)
        );
        ssh_conn
            .execute_shell_interactive(&setup_dir_cmd)
            .with_context(|| {
                format!(
                    "Failed to create .ssh directory for user {} on {}",
                    target_username, host
                )
            })?;

        // Then append the key and set permissions
        // Use write_file pattern - write key content through stdin to avoid all quoting issues
        // Use -t (single TTY) to allow stdin piping while still allowing password prompts
        let mut ssh_args = ssh_conn.build_ssh_args();
        ssh_args.push("-t".to_string()); // Single TTY for password prompts but allow stdin
        ssh_args.push("sh".to_string());
        ssh_args.push("-c".to_string());
        ssh_args.push(format!(
            r#"cat >> {} && chmod 600 {}"#,
            shell_escape(&authorized_keys_path),
            shell_escape(&authorized_keys_path)
        ));

        let mut cmd = Command::new("ssh");
        cmd.args(&ssh_args);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::inherit());

        let mut child = cmd
            .spawn()
            .with_context(|| format!("Failed to spawn SSH command for installing key"))?;

        // Write the key through stdin (same pattern as write_file)
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(pubkey_line.as_bytes())?;
            stdin.write_all(b"\n")?; // Add newline
            stdin.flush()?;
            drop(stdin); // Close stdin so cat knows input is complete
        }

        let status = child
            .wait()
            .with_context(|| format!("Failed to install SSH key on {}", host))?;

        if !status.success() {
            anyhow::bail!("Failed to install SSH key on {}", host);
        }

        println!("✓ SSH key copied successfully to {}", host_str);
        Ok(())
    }
}

pub fn _ssh_to_host(
    hostname: &str,
    user: Option<String>,
    fix_keys: bool,
    copy_keys: bool,
    ssh_args: &[String],
    config: &EnvConfig,
) -> Result<()> {
    // If hostname is empty, list available hosts
    if hostname.is_empty() {
        println!("Available hosts:");
        for (host, _) in &config.hosts {
            println!("  - {}", host);
        }
        anyhow::bail!("Please specify a hostname");
    }

    let host_config = crate::services::host::get_host_config_or_error(hostname)?;

    // Collect all possible host addresses
    let mut all_hosts = Vec::new();
    if let Some(ip) = &host_config.ip {
        all_hosts.push(ip.clone());
    }
    if let Some(hostname) = &host_config.hostname {
        all_hosts.push(hostname.clone());
        all_hosts.push(format!("{}.{}", hostname, config._tailnet_base));
    }

    // If fix_keys is enabled, remove host keys for all possible addresses
    if fix_keys {
        println!("Fix keys mode enabled. Removing host keys for all configured addresses...");
        for host in &all_hosts {
            if _prompt_remove_host_key(host)? {
                _remove_ssh_host_key(host)?;
            }
        }
    }

    let mut tried_hosts = Vec::new();

    // If --keys flag is set, copy SSH key first (will prompt for username if needed)
    if copy_keys {
        let target_host = if let Some(ip) = &host_config.ip {
            ip.as_str()
        } else if let Some(hostname) = &host_config.hostname {
            hostname.as_str()
        } else {
            anyhow::bail!("No IP or Tailscale hostname configured for {}", hostname);
        };

        // For key copying, determine username (prompt if not provided)
        let username_for_keys: Option<String> = if let Some(u) = &user {
            Some(u.clone())
        } else {
            // Prompt for username
            let default_user = config::get_default_username();
            print!("Username (press Enter for '{}'): ", default_user);
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            let input_username = input.trim();
            if input_username.is_empty() {
                Some(default_user)
            } else {
                Some(input_username.to_string())
            }
        };

        // For key copying, we need server username (to SSH in) and target username (where to install key)
        // Prompt for server username, then target username
        // Server username defaults to what we'll use to connect, target username is where key goes
        let default_target_user = config::get_default_username();

        print!(
            "Target username to install key for (press Enter for '{}'): ",
            default_target_user
        );
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let target_username = if input.trim().is_empty() {
            default_target_user
        } else {
            input.trim().to_string()
        };

        copy_ssh_key(
            target_host,
            username_for_keys.as_deref(), // Server username (to SSH into)
            Some(&target_username),       // Target username (where to install key)
        )?;
        // After copying keys, we can proceed with connection
    }

    // Determine username for connection - use provided, or try default first
    let username: Option<String> = if let Some(u) = user {
        Some(u)
    } else {
        // Don't prompt yet - try with default username first via key-based auth
        None // Will use default username from environment when connecting
    };

    let username_ref = username.as_deref();

    // Build list of hosts to try in order
    let mut hosts_to_try: Vec<(String, String)> = Vec::new();

    if let Some(ip) = &host_config.ip {
        hosts_to_try.push((ip.clone(), format!("IP: {}", ip)));
    }

    if let Some(hostname) = &host_config.hostname {
        hosts_to_try.push((hostname.clone(), format!("Hostname: {}", hostname)));
        hosts_to_try.push((
            format!("{}.{}", hostname, config._tailnet_base),
            format!("FQDN: {}.{}", hostname, config._tailnet_base),
        ));
    }

    // Try each host in sequence
    let total_hosts = hosts_to_try.len();
    for (idx, (host, description)) in hosts_to_try.iter().enumerate() {
        tried_hosts.push(host.clone());

        // Try key-based authentication first (silently, no prompts)
        // Use default username first (SSH typically needs a username)
        let default_username = config::get_default_username();

        // Try with default username first
        match _connect_ssh_key_based(host, Some(&default_username), ssh_args) {
            Ok(_) => return Ok(()),
            Err(_) => {} // Key-based auth failed, continue
        }

        // If username was explicitly provided via flag, try that too
        if let Some(ref u) = username {
            if u != &default_username {
                match _connect_ssh_key_based(host, Some(u), ssh_args) {
                    Ok(_) => return Ok(()),
                    Err(_) => {} // Key-based auth failed, continue
                }
            }
        }

        // All key-based auth attempts failed, need password-based auth
        println!("Attempting to connect to {} ({})...", description, host);

        // Use default username for password auth (no prompt needed)
        let final_username = if username_ref.is_none() {
            // Use default username without prompting
            Some(default_username)
        } else {
            username.clone()
        };
        // Try to connect with password authentication as fallback
        // This will allow interactive password prompts
        match _connect_ssh(host, final_username.as_deref(), ssh_args) {
            Ok(_) => {
                // Connection succeeded, we're done
                return Ok(());
            }
            Err(e) => {
                // Connection failed, try next host
                eprintln!("Connection to {} failed: {}", host, e);
                if idx < total_hosts - 1 {
                    println!("Trying next host...");
                }
            }
        }
    }

    // All attempts failed
    eprintln!("✗ Failed to connect to any host");
    eprintln!("  Tried:");
    for host in &tried_hosts {
        eprintln!("    - {}", host);
    }
    std::process::exit(1);
}
