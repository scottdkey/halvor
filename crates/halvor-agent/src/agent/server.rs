use halvor_core::utils::{bytes_to_string, format_bind_address, read_json, write_json};
use anyhow::{Context, Result};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::net::{TcpListener, TcpStream};

/// Halvor Agent Server
/// Runs as a daemon on each host to enable secure remote execution and config sync
pub struct AgentServer {
    port: u16,
    #[allow(dead_code)]
    secret: Option<String>,
}

impl Default for AgentServer {
    fn default() -> Self {
        Self {
            port: 13500,
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
    SyncDatabase {
        /// Hostname of the requesting agent
        from_hostname: String,
        /// Timestamp of last sync (to avoid unnecessary transfers)
        last_sync: Option<i64>,
    },
    Ping,
    /// Request to join the mesh with a token
    JoinRequest {
        join_token: String,
        joiner_hostname: String,
        joiner_public_key: String,
    },
    /// Validate a join token (check if it's valid before attempting join)
    ValidateToken {
        join_token: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AgentResponse {
    Success { output: String },
    Error { message: String },
    HostInfo { info: HostInfo },
    Pong,
    /// Response to join request with shared secret
    JoinAccepted {
        shared_secret: String,
        mesh_peers: Vec<String>,
    },
    /// Response to token validation
    TokenValid {
        issuer_hostname: String,
    },
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
            AgentRequest::SyncDatabase {
                from_hostname,
                last_sync,
            } => self.sync_database(&from_hostname, last_sync)?,
            AgentRequest::JoinRequest {
                join_token,
                joiner_hostname,
                joiner_public_key,
            } => self.handle_join_request(&join_token, &joiner_hostname, &joiner_public_key)?,
            AgentRequest::ValidateToken { join_token } => self.validate_token(&join_token)?,
        };

        // Send response
        write_json(&mut stream, &response)?;

        Ok(())
    }

    fn get_host_info(&self) -> Result<AgentResponse> {
        use crate::apps::tailscale;
        use halvor_core::utils::networking;
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

        // Check provisioning info dynamically (no database)
        use halvor_core::utils::exec::Executor;
        let local_exec = Executor::Local;
        let tailscale_installed = tailscale::is_tailscale_installed(&local_exec);
        // Portainer check would require checking if portainer is running, default to false
        let portainer_installed = false;

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
        _token: &str,
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

    fn sync_config(&self, _data: Vec<u8>) -> Result<AgentResponse> {
        // TODO: Decrypt and apply config sync
        // TODO: Handle conflicts
        Ok(AgentResponse::Success {
            output: "Config synced".to_string(),
        })
    }

    fn sync_database(&self, from_hostname: &str, _last_sync: Option<i64>) -> Result<AgentResponse> {
        use halvor_core::services::host;
        use halvor_db::generated::agent_peers;

        // Export host configs and settings for this host
        let local_hostname = std::env::var("HOSTNAME")
            .or_else(|_| std::fs::read_to_string("/etc/hostname"))
            .unwrap_or_else(|_| "unknown".to_string())
            .trim()
            .to_string();

        // Get all hosts from .env config
        let hosts = host::list_hosts().unwrap_or_default();
        let mut host_configs = std::collections::HashMap::new();
        for hostname in &hosts {
            if let Ok(Some(config)) = host::get_host_config(hostname) {
                host_configs.insert(hostname.clone(), config);
            }
        }

        // Get settings (from environment variables)
        let db_settings: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        // Settings are now in environment variables loaded via direnv from .envrc

        // Get ALL mesh peers from database to share with requesting node
        let peer_rows = agent_peers::select_many(
            "status = ?1",
            &[&"active" as &dyn rusqlite::types::ToSql],
        ).unwrap_or_default();

        let mut mesh_peers = Vec::new();
        for peer in &peer_rows {
            mesh_peers.push(serde_json::json!({
                "hostname": peer.hostname,
                "tailscale_ip": peer.tailscale_ip,
                "tailscale_hostname": peer.tailscale_hostname,
                "public_key": peer.public_key,
                "status": peer.status,
                "last_seen_at": peer.last_seen_at,
                "joined_at": peer.joined_at,
            }));
        }

        // Serialize sync data - includes ALL known peers for self-healing
        let sync_data = serde_json::json!({
            "from_hostname": from_hostname,
            "local_hostname": local_hostname,
            "hosts": host_configs,
            "settings": db_settings,
            "mesh_peers": mesh_peers, // Share all known peers
        });

        let data_str = serde_json::to_string(&sync_data)?;

        Ok(AgentResponse::Success { output: data_str })
    }

    /// Handle a join request from a new agent
    fn handle_join_request(
        &self,
        join_token: &str,
        joiner_hostname: &str,
        joiner_public_key: &str,
    ) -> Result<AgentResponse> {
        use crate::agent::mesh;
        use halvor_core::utils::crypto;

        eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        eprintln!("[AGENT SERVER] Received join request from: {}", joiner_hostname);
        eprintln!("[AGENT SERVER] Token preview (first 50 chars): {}", &join_token[..50.min(join_token.len())]);
        eprintln!("[AGENT SERVER] Public key: {}", joiner_public_key);

        // Validate the join token
        eprintln!("[AGENT SERVER] Starting token validation...");
        let _token = match mesh::validate_join_token(join_token) {
            Ok(t) => {
                eprintln!("[AGENT SERVER] ✓ Token validation successful");
                t
            },
            Err(e) => {
                eprintln!("[AGENT SERVER] ✗ Token validation failed: {}", e);
                eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                return Ok(AgentResponse::Error {
                    message: format!("Invalid join token: {}", e),
                });
            }
        };

        // Generate a shared secret for this peer
        eprintln!("[AGENT SERVER] Generating shared secret for peer...");
        let shared_secret_bytes = crypto::generate_random_key()?;
        let shared_secret = base64::engine::general_purpose::STANDARD.encode(&shared_secret_bytes);

        // Add peer to the mesh
        eprintln!("[AGENT SERVER] Adding peer to mesh database...");
        if let Err(e) = mesh::add_peer(
            joiner_hostname,
            None, // Will be updated when peer is discovered
            None,
            joiner_public_key,
            &shared_secret,
        ) {
            eprintln!("[AGENT SERVER] ✗ Failed to add peer: {}", e);
            eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            return Ok(AgentResponse::Error {
                message: format!("Failed to add peer: {}", e),
            });
        }
        eprintln!("[AGENT SERVER] ✓ Peer added to mesh");

        // Mark token as used
        eprintln!("[AGENT SERVER] Marking token as used...");
        if let Err(e) = mesh::mark_token_used(join_token, joiner_hostname) {
            eprintln!("[AGENT SERVER] ⚠ Warning: Failed to mark token as used: {}", e);
        } else {
            eprintln!("[AGENT SERVER] ✓ Token marked as used");
        }

        // Get current mesh peers
        let peers = mesh::get_active_peers().unwrap_or_default();
        eprintln!("[AGENT SERVER] Current mesh has {} peer(s)", peers.len());

        // Broadcast new peer to all existing peers in the mesh
        eprintln!("[AGENT SERVER] Broadcasting new peer to existing mesh members...");
        let broadcast_count = self.broadcast_new_peer_to_mesh(joiner_hostname, &peers);
        eprintln!("[AGENT SERVER] Notified {} existing peer(s)", broadcast_count);

        eprintln!("[AGENT SERVER] ✓ Join accepted! Mesh now has {} peer(s)", peers.len() + 1);
        eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        Ok(AgentResponse::JoinAccepted {
            shared_secret,
            mesh_peers: peers,
        })
    }

    /// Broadcast new peer information to all existing peers in the mesh
    fn broadcast_new_peer_to_mesh(&self, new_peer_hostname: &str, existing_peers: &[String]) -> usize {
        use crate::agent::api::AgentClient;
        use crate::agent::discovery::HostDiscovery;

        let discovery = HostDiscovery::default();
        let Ok(hosts) = discovery.discover_all() else {
            eprintln!("[AGENT SERVER] Failed to discover hosts for broadcast");
            return 0;
        };

        let mut notified = 0;

        for peer_hostname in existing_peers {
            // Find the host info for this peer
            let host_info = hosts.iter().find(|h| &h.hostname == peer_hostname);

            if let Some(host) = host_info {
                if !host.reachable {
                    eprintln!("[AGENT SERVER]   {} (unreachable, skipping)", peer_hostname);
                    continue;
                }

                let ip = host.tailscale_ip.as_ref().or(host.local_ip.as_ref());
                if let Some(ip) = ip {
                    let client = AgentClient::new(ip, host.agent_port);

                    // Notify peer about the new node via sync
                    match client.sync_database(new_peer_hostname, None) {
                        Ok(_) => {
                            eprintln!("[AGENT SERVER]   ✓ Notified {}", peer_hostname);
                            notified += 1;
                        }
                        Err(e) => {
                            eprintln!("[AGENT SERVER]   ✗ Failed to notify {}: {}", peer_hostname, e);
                        }
                    }
                } else {
                    eprintln!("[AGENT SERVER]   {} (no IP, skipping)", peer_hostname);
                }
            } else {
                eprintln!("[AGENT SERVER]   {} (not found, skipping)", peer_hostname);
            }
        }

        notified
    }

    /// Validate a join token without consuming it
    fn validate_token(&self, join_token: &str) -> Result<AgentResponse> {
        use crate::agent::mesh;

        match mesh::validate_join_token(join_token) {
            Ok(token) => Ok(AgentResponse::TokenValid {
                issuer_hostname: token.issuer_hostname,
            }),
            Err(e) => Ok(AgentResponse::Error {
                message: format!("Invalid token: {}", e),
            }),
        }
    }
}
