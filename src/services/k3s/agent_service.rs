//! Systemd service management for halvor agent
use crate::utils::exec::CommandExecutor;
use anyhow::{Context, Result};
use std::path::PathBuf;

/// Create and enable halvor agent systemd service
pub fn setup_agent_service<E: CommandExecutor>(exec: &E, web_port: Option<u16>) -> Result<()> {
    println!("Setting up halvor agent systemd service...");

    // Check if agent is already running as a daemon and stop it
    println!("Checking for existing halvor agent daemon...");
    let pid_file_path = format!("{}/.config/halvor/halvor-agent.pid", 
        exec.get_home_dir().unwrap_or_else(|_| "/root".to_string()));
    
    // Check if PID file exists and process is running
    let pid_check = exec.execute_shell(&format!(
        "test -f {} && kill -0 $(cat {}) 2>/dev/null && echo running || echo not_running",
        pid_file_path, pid_file_path
    )).ok();
    
    if let Some(check) = pid_check {
        let status_str = String::from_utf8_lossy(&check.stdout);
        let status = status_str.trim();
        if status == "running" {
            println!("  Found running halvor agent daemon, stopping it...");
            let _ = exec.execute_shell(&format!("kill $(cat {}) 2>/dev/null || true", pid_file_path));
            std::thread::sleep(std::time::Duration::from_secs(1));
            println!("  ✓ Stopped existing daemon");
        }
    }
    
    // Also check for systemd service and stop it if running
    let service_check = exec.execute_shell("systemctl is-active halvor-agent.service 2>/dev/null || echo inactive").ok();
    if let Some(check) = service_check {
        let status_str = String::from_utf8_lossy(&check.stdout);
        let status = status_str.trim();
        if status == "active" {
            println!("  Found running halvor-agent systemd service, stopping it...");
            let _ = exec.execute_shell("systemctl stop halvor-agent.service 2>/dev/null || true");
            std::thread::sleep(std::time::Duration::from_secs(1));
            println!("  ✓ Stopped existing systemd service");
        }
    }

    // Get halvor binary path
    let halvor_path = exec
        .execute_shell("which halvor || echo /usr/local/bin/halvor")
        .and_then(|o| {
            let path = String::from_utf8(o.stdout)?;
            Ok(path.trim().to_string())
        })
        .unwrap_or_else(|_| "/usr/local/bin/halvor".to_string());

    // Get web directory (if available)
    let web_dir = std::env::var("HALVOR_WEB_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/opt/halvor/halvor-web"));

    // Get config directory for PID file (must match what agent.rs uses)
    // The agent uses get_agent_pid_file() which uses config_manager::get_config_dir()
    // This resolves to ~/.config/halvor/halvor-agent.pid (or /root/.config/halvor when running as root)
    let home_dir = exec.get_home_dir().unwrap_or_else(|_| "/root".to_string());
    let config_dir = format!("{}/.config/halvor", home_dir);
    let pid_file = format!("{}/halvor-agent.pid", config_dir);
    
    // Ensure config directory exists
    exec.execute_shell(&format!("mkdir -p {}", config_dir))
        .context("Failed to create config directory")?;

    // Build service file content
    // Use Type=forking with --daemon flag for traditional daemon behavior
    let service_content = format!(
        r#"[Unit]
Description=Halvor Agent - Secure cluster management service
After=network.target tailscale.service
Wants=network.target

[Service]
Type=forking
User=root
ExecStart={} agent start --port 13500{} --daemon
PIDFile={}
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

# Environment variables
Environment="HALVOR_DB_DIR=/var/lib/halvor/data"
Environment="HALVOR_WEB_DIR={}"

# Security settings
NoNewPrivileges=true
PrivateTmp=true

[Install]
WantedBy=multi-user.target
"#,
        halvor_path,
        if let Some(wp) = web_port {
            format!(" --web-port {}", wp)
        } else {
            String::new()
        },
        pid_file,
        web_dir.display()
    );

    // Write service file
    let service_file = "/etc/systemd/system/halvor-agent.service";
    exec.write_file(service_file, service_content.as_bytes())
        .context("Failed to write systemd service file")?;

    println!("✓ Service file created: {}", service_file);

    // Reload systemd
    exec.execute_shell("systemctl daemon-reload")
        .context("Failed to reload systemd")?;

    // Enable service
    exec.execute_shell("systemctl enable halvor-agent.service")
        .context("Failed to enable halvor-agent service")?;

    // Start service
    exec.execute_shell("systemctl start halvor-agent.service")
        .context("Failed to start halvor-agent service")?;

    println!("✓ Halvor agent service enabled and started");
    println!("  Use 'systemctl status halvor-agent' to check status");
    println!("  Use 'journalctl -u halvor-agent -f' to view logs");

    Ok(())
}

/// Check if halvor agent service is running
pub fn is_agent_service_running<E: CommandExecutor>(exec: &E) -> Result<bool> {
    let output = exec
        .execute_shell("systemctl is-active halvor-agent.service 2>/dev/null || echo inactive")?;
    let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(status == "active")
}

