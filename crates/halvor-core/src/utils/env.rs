// Environment loading utilities
// Provides functions to load environment variables from direnv (.envrc)

use std::path::Path;
use std::process::Command;

/// Check if .envrc exists in the given directory
pub fn has_envrc(dir: &Path) -> bool {
    dir.join(".envrc").exists()
}

/// Create a Command that will run with direnv environment variables loaded
///
/// If .envrc exists in the base_dir, wraps the command with `direnv exec`
/// to ensure all environment variables from .envrc are available.
///
/// # Arguments
/// * `base_dir` - The directory containing .envrc (usually the project root)
/// * `command` - The command to run (e.g., "fastlane")
/// * `args` - Arguments to pass to the command
/// * `working_dir` - The directory to run the command in (can be different from base_dir)
///
/// # Returns
/// A configured Command that will have direnv environment variables loaded
pub fn command_with_direnv(
    base_dir: &Path,
    command: &str,
    args: &[&str],
    working_dir: Option<&Path>,
) -> Command {
    let working_dir = working_dir.unwrap_or(base_dir);

    if has_envrc(base_dir) {
        // Use direnv exec to load environment variables
        // direnv exec <base_dir> <command> [args...]
        let mut cmd = Command::new("direnv");
        cmd.arg("exec").arg(base_dir);

        // If working_dir differs from base_dir, wrap in bash to cd first
        if working_dir != base_dir {
            let cd_and_run = format!(
                "cd {} && {} {}",
                working_dir.display(),
                command,
                args.join(" ")
            );
            cmd.arg("bash").arg("-c").arg(cd_and_run);
        } else {
            cmd.arg(command);
            cmd.args(args);
        }

        cmd
    } else {
        // Fallback: run command directly without direnv
        let mut cmd = Command::new(command);
        cmd.args(args);
        cmd.current_dir(working_dir);
        cmd
    }
}

/// Create a Command that will run a shell command with direnv environment variables loaded
///
/// This is useful when you need to run complex shell commands (e.g., with pipes, redirects)
/// that require direnv environment variables.
///
/// # Arguments
/// * `base_dir` - The directory containing .envrc (usually the project root)
/// * `shell_command` - The full shell command to execute (e.g., "cd fastlane && fastlane ios build")
/// * `working_dir` - Optional working directory (defaults to base_dir)
///
/// # Returns
/// A configured Command that will have direnv environment variables loaded
pub fn shell_command_with_direnv(
    base_dir: &Path,
    shell_command: &str,
    working_dir: Option<&Path>,
) -> Command {
    let working_dir = working_dir.unwrap_or(base_dir);

    if has_envrc(base_dir) {
        // Use direnv exec to load environment variables, then run shell command
        let mut cmd = Command::new("direnv");
        cmd.arg("exec")
            .arg(base_dir)
            .arg("bash")
            .arg("-c")
            .arg(shell_command)
            .current_dir(working_dir);

        cmd
    } else {
        // Fallback: run shell command directly without direnv
        let mut cmd = Command::new("bash");
        cmd.arg("-c").arg(shell_command);
        cmd.current_dir(working_dir);
        cmd
    }
}
