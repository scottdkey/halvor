use crate::utils::crypto;
use crate::utils::{bytes_to_string, format_bind_address, read_json, write_json};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

/// Halvor Agent Server
/// Runs as a daemon on each host to enable secure remote execution and config sync
pub struct AgentServer {
    port: u16,
    secret: Option<String>,
}

impl Default for AgentServer {
    fn default() -> Self {
        Self {
            port: 23500,
            secret: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AgentRequest {
    ExecuteCommand {
        command: String,
        args: Vec<String>,
        token: String,
    },
    GetHostInfo,
    SyncConfig {
        data: Vec<u8>,
    },
    Ping,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AgentResponse {
    Success { output: String },
    Error { message: String },
    HostInfo { info: HostInfo },
    Pong,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HostInfo {
    pub hostname: String,
    pub local_ip: Option<String>,
    pub tailscale_ip: Option<String>,
    pub tailscale_hostname: Option<String>,
    pub docker_version: Option<String>,
    pub tailscale_installed: bool,
    pub portainer_installed: bool,
}

impl AgentServer {
    pub fn new(port: u16, secret: Option<String>) -> Self {
        Self { port, secret }
    }

    /// Start the agent server
    pub fn start(&self) -> Result<()> {
        let addr = format_bind_address(self.port);
        let listener =
            TcpListener::bind(&addr).with_context(|| format!("Failed to bind to {}", addr))?;

        println!("Halvor agent listening on port {}", self.port);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    if let Err(e) = self.handle_connection(stream) {
                        eprintln!("Error handling connection: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("Error accepting connection: {}", e);
                }
            }
        }

        Ok(())
    }

    fn handle_connection(&self, mut stream: TcpStream) -> Result<()> {
        // Read request
        let request: AgentRequest = read_json(&mut stream, 4096)?;

        // Handle request
        let response = match request {
            AgentRequest::Ping => AgentResponse::Pong,
            AgentRequest::GetHostInfo => self.get_host_info()?,
            AgentRequest::ExecuteCommand {
                command,
                args,
                token,
            } => self.execute_command(&command, &args, &token)?,
            AgentRequest::SyncConfig { data } => self.sync_config(data)?,
        };

        // Send response
        write_json(&mut stream, &response)?;

        Ok(())
    }

    fn get_host_info(&self) -> Result<AgentResponse> {
        use crate::db;
        use crate::services::tailscale;
        use crate::utils::networking;
        use std::env;

        let hostname = env::var("HOSTNAME")
            .or_else(|_| std::fs::read_to_string("/etc/hostname"))
            .unwrap_or_else(|_| "unknown".to_string())
            .trim()
            .to_string();

        let local_ips = networking::get_local_ips().ok();
        let local_ip = local_ips.and_then(|ips| ips.first().cloned());

        // Try to get Tailscale info
        let tailscale_ip = tailscale::get_tailscale_ip().ok().flatten();
        let tailscale_hostname = tailscale::get_tailscale_hostname().ok().flatten();

        // Get Docker version
        let docker_version = std::process::Command::new("docker")
            .args(&["version", "--format", "{{.Server.Version}}"])
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    String::from_utf8(output.stdout)
                        .ok()
                        .map(|s| s.trim().to_string())
                } else {
                    None
                }
            });

        // Get provisioning info from DB
        let (tailscale_installed, portainer_installed) =
            if let Ok(Some(info)) = db::get_host_info(&hostname) {
                (info.2, info.3)
            } else {
                (false, false)
            };

        Ok(AgentResponse::HostInfo {
            info: HostInfo {
                hostname,
                local_ip,
                tailscale_ip,
                tailscale_hostname,
                docker_version,
                tailscale_installed,
                portainer_installed,
            },
        })
    }

    fn execute_command(
        &self,
        command: &str,
        args: &[String],
        token: &str,
    ) -> Result<AgentResponse> {
        // TODO: Validate token
        // TODO: Check permissions
        // TODO: Execute command safely

        use std::process::Command;
        let output = Command::new(command)
            .args(args)
            .output()
            .with_context(|| format!("Failed to execute command: {}", command))?;

        let stdout = bytes_to_string(&output.stdout);
        let stderr = bytes_to_string(&output.stderr);

        if output.status.success() {
            Ok(AgentResponse::Success {
                output: stdout.to_string(),
            })
        } else {
            Ok(AgentResponse::Error {
                message: format!("Command failed: {}", stderr),
            })
        }
    }

    fn sync_config(&self, data: Vec<u8>) -> Result<AgentResponse> {
        // TODO: Decrypt and apply config sync
        // TODO: Handle conflicts
        Ok(AgentResponse::Success {
            output: "Config synced".to_string(),
        })
    }
}
