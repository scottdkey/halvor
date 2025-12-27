//! Service management for halvor agent (systemd on Linux, launchd on macOS)
use crate::utils::exec::CommandExecutor;
use anyhow::{Context, Result};
use std::path::PathBuf;

/// Detect if the target system is macOS
fn is_macos<E: CommandExecutor>(exec: &E) -> bool {
    exec.execute_shell("uname -s")
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_lowercase() == "darwin")
        .unwrap_or(false)
}

/// Create and enable halvor agent service (auto-detects platform)
pub fn setup_agent_service<E: CommandExecutor>(exec: &E, web_port: Option<u16>) -> Result<()> {
    if is_macos(exec) {
        setup_agent_service_macos(exec, web_port)
    } else {
        setup_agent_service_linux(exec, web_port)
    }
}

/// Create and enable halvor agent launchd service on macOS
fn setup_agent_service_macos<E: CommandExecutor>(exec: &E, web_port: Option<u16>) -> Result<()> {
    println!("Setting up halvor agent launchd service...");

    let home_dir = exec.get_home_dir().unwrap_or_else(|_| "/root".to_string());
    let plist_path = format!("{}/Library/LaunchAgents/com.halvor.agent.plist", home_dir);
    let log_dir = format!("{}/Library/Logs/halvor", home_dir);
    let config_dir = format!("{}/.config/halvor", home_dir);

    // Check if service is already loaded and running
    let service_loaded = exec
        .execute_shell("launchctl list com.halvor.agent 2>/dev/null")
        .ok()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if service_loaded {
        // Check if it's actually running (PID column is not "-")
        let service_running = exec
            .execute_shell("launchctl list com.halvor.agent 2>/dev/null | awk '{print $1}'")
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| {
                let pid = s.trim();
                !pid.is_empty() && pid != "-"
            })
            .unwrap_or(false);

        if service_running {
            println!("✓ Halvor agent service is already running");
            println!("  Skipping setup (service already active)");
            return Ok(());
        } else {
            println!("⚠️  Service is loaded but not running, attempting to start...");
            let start_result = exec.execute_shell("launchctl start com.halvor.agent");
            if start_result.is_ok() && start_result.unwrap().status.success() {
                println!("✓ Started halvor agent service");
                return Ok(());
            }
            // If start failed, unload and continue with full setup
            let _ = exec.execute_shell("launchctl unload -w \"$HOME/Library/LaunchAgents/com.halvor.agent.plist\" 2>/dev/null");
        }
    }

    // Check if agent is already running as a daemon and stop it
    println!("Checking for existing halvor agent daemon...");
    let pid_file_path = format!("{}/halvor-agent.pid", config_dir);

    // Check if PID file exists and process is running
    let pid_check = exec
        .execute_shell(&format!(
            "test -f {} && kill -0 $(cat {}) 2>/dev/null && echo running || echo not_running",
            pid_file_path, pid_file_path
        ))
        .ok();

    if let Some(check) = pid_check {
        let status_str = String::from_utf8_lossy(&check.stdout);
        let status = status_str.trim();
        if status == "running" {
            println!("  Found running halvor agent daemon, stopping it...");
            let _ = exec.execute_shell(&format!(
                "kill $(cat {}) 2>/dev/null || true",
                pid_file_path
            ));
            std::thread::sleep(std::time::Duration::from_secs(1));
            println!("  ✓ Stopped existing daemon");
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
        .unwrap_or_else(|_| PathBuf::from("/opt/halvor/projects/web"));

    // Ensure directories exist
    exec.execute_shell(&format!("mkdir -p {}", config_dir))
        .context("Failed to create config directory")?;
    exec.execute_shell(&format!("mkdir -p {}", log_dir))
        .context("Failed to create log directory")?;
    exec.execute_shell(&format!("mkdir -p {}/Library/LaunchAgents", home_dir))
        .context("Failed to create LaunchAgents directory")?;

    // Build port argument
    let port_arg = if let Some(wp) = web_port {
        format!("--web-port\n        <string>{}</string>", wp)
    } else {
        String::new()
    };

    // Build launchd plist content
    let plist_content = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.halvor.agent</string>

    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
        <string>agent</string>
        <string>start</string>
        <string>--port</string>
        <string>13500</string>{}
    </array>

    <key>RunAtLoad</key>
    <true/>

    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>

    <key>ThrottleInterval</key>
    <integer>10</integer>

    <key>StandardOutPath</key>
    <string>{}/halvor-agent.log</string>

    <key>StandardErrorPath</key>
    <string>{}/halvor-agent.error.log</string>

    <key>EnvironmentVariables</key>
    <dict>
        <key>HALVOR_DB_DIR</key>
        <string>{}/.local/share/halvor/data</string>
        <key>HALVOR_WEB_DIR</key>
        <string>{}</string>
    </dict>

    <key>WorkingDirectory</key>
    <string>{}</string>
</dict>
</plist>
"#,
        halvor_path,
        if port_arg.is_empty() {
            String::new()
        } else {
            format!("\n        <string>{}</string>", port_arg)
        },
        log_dir,
        log_dir,
        home_dir,
        web_dir.display(),
        home_dir
    );

    // Write plist file
    exec.write_file(&plist_path, plist_content.as_bytes())
        .context("Failed to write launchd plist file")?;

    println!("✓ Plist file created: {}", plist_path);

    // Load the service
    exec.execute_shell(&format!("launchctl load -w \"{}\"", plist_path))
        .context("Failed to load halvor-agent service")?;

    // Start the service
    exec.execute_shell("launchctl start com.halvor.agent")
        .context("Failed to start halvor-agent service")?;

    println!("✓ Halvor agent service loaded and started");
    println!("  Use 'launchctl list com.halvor.agent' to check status");
    println!(
        "  View logs: tail -f {}/halvor-agent.log",
        log_dir
    );

    Ok(())
}

/// Create and enable halvor agent systemd service on Linux
fn setup_agent_service_linux<E: CommandExecutor>(exec: &E, web_port: Option<u16>) -> Result<()> {
    println!("Setting up halvor agent systemd service...");

    // Check if service is already configured and running
    let service_file = "/etc/systemd/system/halvor-agent.service";
    let service_exists = exec.file_exists(service_file).unwrap_or(false);
    let service_enabled = exec
        .execute_shell("systemctl is-enabled halvor-agent.service 2>/dev/null || echo disabled")
        .ok()
        .and_then(|o| {
            String::from_utf8(o.stdout).ok().map(|s| s.trim() == "enabled")
        })
        .unwrap_or(false);
    let service_active = exec
        .execute_shell("systemctl is-active halvor-agent.service 2>/dev/null || echo inactive")
        .ok()
        .and_then(|o| {
            String::from_utf8(o.stdout).ok().map(|s| s.trim() == "active")
        })
        .unwrap_or(false);

    if service_exists && service_enabled {
        if service_active {
            println!("✓ Halvor agent service is already configured and running");
            println!("  Skipping setup (service already active)");
            return Ok(());
        } else {
            println!("⚠️  Service is configured but not running, attempting to start...");
            let start_result = exec.execute_shell("sudo systemctl start halvor-agent.service");
            if start_result.is_ok() && start_result.unwrap().status.success() {
                println!("✓ Started halvor agent service");
                return Ok(());
            }
            // If start failed, continue with full setup
        }
    }

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
            .unwrap_or_else(|_| PathBuf::from("/opt/halvor/projects/web"));

    // Get the current user (who is running the setup)
    let current_user = exec
        .execute_shell("whoami")
        .and_then(|o| {
            let user = String::from_utf8(o.stdout)?;
            Ok(user.trim().to_string())
        })
        .unwrap_or_else(|_| "root".to_string());

    // Get user's home directory
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
User={}
Group={}
ExecStart={} agent start --port 13500{} --daemon
PIDFile={}
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

# Environment variables
Environment="HOME={}"
Environment="HALVOR_DB_DIR={}"
Environment="HALVOR_WEB_DIR={}"

# Security settings
NoNewPrivileges=true
PrivateTmp=true

[Install]
WantedBy=multi-user.target
"#,
        current_user,
        current_user,
        halvor_path,
        if let Some(wp) = web_port {
            format!(" --web-port {}", wp)
        } else {
            String::new()
        },
        pid_file,
        home_dir,
        config_dir,
        web_dir.display()
    );

    // Write service file
    let service_file = "/etc/systemd/system/halvor-agent.service";
    exec.write_file(service_file, service_content.as_bytes())
        .context("Failed to write systemd service file")?;

    println!("✓ Service file created: {}", service_file);

    // Reload systemd
    exec.execute_shell("sudo systemctl daemon-reload")
        .context("Failed to reload systemd")?;

    // Enable service
    exec.execute_shell("sudo systemctl enable halvor-agent.service")
        .context("Failed to enable halvor-agent service")?;

    // Start service
    exec.execute_shell("sudo systemctl start halvor-agent.service")
        .context("Failed to start halvor-agent service")?;

    println!("✓ Halvor agent service enabled and started");
    println!("  Use 'systemctl status halvor-agent' to check status");
    println!("  Use 'journalctl -u halvor-agent -f' to view logs");

    Ok(())
}

/// Check if halvor agent service is running (auto-detects platform)
#[allow(dead_code)]
pub fn is_agent_service_running<E: CommandExecutor>(exec: &E) -> Result<bool> {
    if is_macos(exec) {
        is_agent_service_running_macos(exec)
    } else {
        is_agent_service_running_linux(exec)
    }
}

/// Check if halvor agent service is running on macOS
#[allow(dead_code)]
fn is_agent_service_running_macos<E: CommandExecutor>(exec: &E) -> Result<bool> {
    let output = exec
        .execute_shell("launchctl list com.halvor.agent 2>/dev/null | awk '{print $1}'")?;
    let pid = String::from_utf8_lossy(&output.stdout).trim().to_string();
    // If the service is running, the first column will be a PID number, not "-"
    Ok(!pid.is_empty() && pid != "-" && pid.parse::<u32>().is_ok())
}

/// Check if halvor agent service is running on Linux
#[allow(dead_code)]
fn is_agent_service_running_linux<E: CommandExecutor>(exec: &E) -> Result<bool> {
    let output = exec
        .execute_shell("systemctl is-active halvor-agent.service 2>/dev/null || echo inactive")?;
    let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(status == "active")
}

/// Stop halvor agent service (auto-detects platform)
pub fn stop_agent_service<E: CommandExecutor>(exec: &E) -> Result<()> {
    if is_macos(exec) {
        stop_agent_service_macos(exec)
    } else {
        stop_agent_service_linux(exec)
    }
}

/// Stop halvor agent service on macOS
fn stop_agent_service_macos<E: CommandExecutor>(exec: &E) -> Result<()> {
    // Check if service is loaded
    let service_loaded = exec
        .execute_shell("launchctl list com.halvor.agent 2>/dev/null")
        .ok()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if service_loaded {
        println!("Stopping halvor agent service...");
        let _ = exec.execute_shell("launchctl stop com.halvor.agent 2>/dev/null");
        // Give it a moment to stop
        std::thread::sleep(std::time::Duration::from_secs(1));
        println!("✓ Agent service stopped");
    }

    // Also check for daemon process
    let home_dir = exec.get_home_dir().unwrap_or_else(|_| "/root".to_string());
    let pid_file_path = format!("{}/.config/halvor/halvor-agent.pid", home_dir);

    let pid_check = exec
        .execute_shell(&format!(
            "test -f {} && kill -0 $(cat {}) 2>/dev/null && echo running || echo not_running",
            pid_file_path, pid_file_path
        ))
        .ok();

    if let Some(check) = pid_check {
        let status_str = String::from_utf8_lossy(&check.stdout);
        if status_str.trim() == "running" {
            println!("Stopping halvor agent daemon...");
            let _ = exec.execute_shell(&format!(
                "kill $(cat {}) 2>/dev/null || true",
                pid_file_path
            ));
            std::thread::sleep(std::time::Duration::from_secs(1));
            println!("✓ Agent daemon stopped");
        }
    }

    Ok(())
}

/// Stop halvor agent service on Linux
fn stop_agent_service_linux<E: CommandExecutor>(exec: &E) -> Result<()> {
    // Check systemd service
    let service_active = exec
        .execute_shell("systemctl is-active halvor-agent.service 2>/dev/null || echo inactive")
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim() == "active")
        .unwrap_or(false);

    if service_active {
        println!("Stopping halvor agent service...");
        let _ = exec.execute_shell("sudo systemctl stop halvor-agent.service 2>/dev/null");
        std::thread::sleep(std::time::Duration::from_secs(1));
        println!("✓ Agent service stopped");
    }

    // Also check for daemon process
    let home_dir = exec.get_home_dir().unwrap_or_else(|_| "/root".to_string());
    let pid_file_path = format!("{}/.config/halvor/halvor-agent.pid", home_dir);

    let pid_check = exec
        .execute_shell(&format!(
            "test -f {} && kill -0 $(cat {}) 2>/dev/null && echo running || echo not_running",
            pid_file_path, pid_file_path
        ))
        .ok();

    if let Some(check) = pid_check {
        let status_str = String::from_utf8_lossy(&check.stdout);
        if status_str.trim() == "running" {
            println!("Stopping halvor agent daemon...");
            let _ = exec.execute_shell(&format!(
                "kill $(cat {}) 2>/dev/null || true",
                pid_file_path
            ));
            std::thread::sleep(std::time::Duration::from_secs(1));
            println!("✓ Agent daemon stopped");
        }
    }

    Ok(())
}

/// Restart halvor agent service (auto-detects platform)
pub fn restart_agent_service<E: CommandExecutor>(exec: &E, web_port: Option<u16>) -> Result<()> {
    if is_macos(exec) {
        restart_agent_service_macos(exec)
    } else {
        restart_agent_service_linux(exec, web_port)
    }
}

/// Restart halvor agent service on macOS
fn restart_agent_service_macos<E: CommandExecutor>(exec: &E) -> Result<()> {
    // Check if service plist exists
    let home_dir = exec.get_home_dir().unwrap_or_else(|_| "/root".to_string());
    let plist_path = format!("{}/Library/LaunchAgents/com.halvor.agent.plist", home_dir);

    if !exec.file_exists(&plist_path).unwrap_or(false) {
        println!("Agent service not configured. Run 'halvor init' to set up the service.");
        return Ok(());
    }

    // Unload and reload to pick up any binary changes
    println!("Restarting halvor agent service...");
    let _ = exec.execute_shell(&format!("launchctl unload \"{}\" 2>/dev/null", plist_path));
    std::thread::sleep(std::time::Duration::from_millis(500));
    exec.execute_shell(&format!("launchctl load -w \"{}\"", plist_path))
        .context("Failed to reload agent service")?;
    exec.execute_shell("launchctl start com.halvor.agent")
        .context("Failed to start agent service")?;

    println!("✓ Agent service restarted");
    Ok(())
}

/// Restart halvor agent service on Linux
fn restart_agent_service_linux<E: CommandExecutor>(exec: &E, _web_port: Option<u16>) -> Result<()> {
    // Check if service exists
    let service_exists = exec.file_exists("/etc/systemd/system/halvor-agent.service").unwrap_or(false);

    if !service_exists {
        println!("Agent service not configured. Run 'halvor init' to set up the service.");
        return Ok(());
    }

    println!("Restarting halvor agent service...");
    // Reload daemon in case binary changed
    let _ = exec.execute_shell("sudo systemctl daemon-reload");
    exec.execute_shell("sudo systemctl restart halvor-agent.service")
        .context("Failed to restart agent service")?;

    println!("✓ Agent service restarted");
    Ok(())
}

