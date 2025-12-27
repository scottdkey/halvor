use crate::agent::server::{AgentRequest, AgentResponse, HostInfo};
use halvor_core::utils::{format_address, read_json, write_json};
use anyhow::{Context, Result};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

/// Client for communicating with halvor agents
pub struct AgentClient {
    host: String,
    port: u16,
    token: Option<String>,
}

impl AgentClient {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            host: host.to_string(),
            port,
            token: None,
        }
    }

    pub fn with_token(mut self, token: &str) -> Self {
        self.token = Some(token.to_string());
        self
    }

    /// Ping the agent
    pub fn ping(&self) -> Result<bool> {
        let response = self.send_request(AgentRequest::Ping)?;
        Ok(matches!(response, AgentResponse::Pong))
    }

    /// Get host information
    pub fn get_host_info(&self) -> Result<HostInfo> {
        let response = self.send_request(AgentRequest::GetHostInfo)?;
        match response {
            AgentResponse::HostInfo { info } => Ok(info),
            AgentResponse::Error { message } => anyhow::bail!("Agent error: {}", message),
            _ => anyhow::bail!("Unexpected response type"),
        }
    }

    /// Execute a command remotely
    pub fn execute_command(&self, command: &str, args: &[&str]) -> Result<String> {
        let token = self.token.as_deref().unwrap_or("default");
        let args_vec: Vec<String> = args.iter().map(|s| s.to_string()).collect();

        let response = self.send_request(AgentRequest::ExecuteCommand {
            command: command.to_string(),
            args: args_vec,
            token: token.to_string(),
        })?;

        match response {
            AgentResponse::Success { output } => Ok(output),
            AgentResponse::Error { message } => anyhow::bail!("Command failed: {}", message),
            _ => anyhow::bail!("Unexpected response type"),
        }
    }

    /// Sync database with remote agent
    pub fn sync_database(&self, from_hostname: &str, last_sync: Option<i64>) -> Result<String> {
        let response = self.send_request(AgentRequest::SyncDatabase {
            from_hostname: from_hostname.to_string(),
            last_sync,
        })?;

        match response {
            AgentResponse::Success { output } => Ok(output),
            AgentResponse::Error { message } => anyhow::bail!("Sync failed: {}", message),
            _ => anyhow::bail!("Unexpected response type"),
        }
    }

    fn send_request(&self, request: AgentRequest) -> Result<AgentResponse> {
        let addr = format_address(&self.host, self.port);
        
        // Resolve address and connect with timeout
        let socket_addr = addr
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| anyhow::anyhow!("Failed to resolve address: {}", addr))?;
        
        let mut stream = TcpStream::connect_timeout(&socket_addr, Duration::from_secs(2))
            .with_context(|| format!("Failed to connect to agent at {} (timeout: 2s)", addr))?;

        write_json(&mut stream, &request)?;
        read_json(&mut stream, 8192)
    }
}
