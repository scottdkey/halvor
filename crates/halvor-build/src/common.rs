// Common build utilities
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

/// Execute a command and check its status, returning an error if it fails
pub fn execute_command(mut cmd: Command, error_msg: &str) -> Result<()> {
    let error_msg = error_msg.to_string();
    let status = cmd.status().with_context(|| error_msg.clone())?;
    if !status.success() {
        anyhow::bail!("{}", error_msg);
    }
    Ok(())
}

/// Execute a command and return its output
pub fn execute_command_output(mut cmd: Command, error_msg: &str) -> Result<std::process::Output> {
    let error_msg = error_msg.to_string();
    cmd.output().with_context(|| error_msg)
}

/// Check if a path exists, returning an error if it doesn't
pub fn ensure_path_exists(path: &PathBuf, error_msg: &str) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("{}", error_msg);
    }
    Ok(())
}

/// Create a directory and all parent directories if they don't exist
pub fn ensure_dir_exists(path: &PathBuf) -> Result<()> {
    std::fs::create_dir_all(path)
        .with_context(|| format!("Failed to create directory: {}", path.display()))
}

/// Copy a file from source to destination
pub fn copy_file(src: &PathBuf, dst: &PathBuf) -> Result<()> {
    std::fs::copy(src, dst).with_context(|| {
        format!(
            "Failed to copy file from {} to {}",
            src.display(),
            dst.display()
        )
    })?;
    Ok(())
}

/// Get the binary name for a target (with .exe for Windows)
pub fn get_binary_name(target: &str) -> &str {
    if target.contains("windows") {
        "halvor.exe"
    } else {
        "halvor"
    }
}

/// Get the binary path for a target
pub fn get_binary_path(target: &str, release: bool) -> PathBuf {
    PathBuf::from("target")
        .join(target)
        .join(if release { "release" } else { "debug" })
        .join(get_binary_name(target))
}

