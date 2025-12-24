//! Process management utilities

use anyhow::Result;
use std::process::Command;

/// Check if a process is running by PID
pub fn is_process_running(pid: u32) -> bool {
    nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid as i32), None).is_ok()
}

/// Find process ID by command pattern
pub fn find_process_by_pattern(pattern: &str) -> Result<Option<u32>> {
    let output = Command::new("pgrep").args(&["-f", pattern]).output()?;

    if output.status.success() {
        let pid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if let Ok(pid) = pid_str.parse::<u32>() {
            return Ok(Some(pid));
        }
    }
    Ok(None)
}
