//! API Definition - Single source of truth for all API endpoints
//! This is used to generate routes in Rust and client libraries for TypeScript, Kotlin, and Swift

use serde::{Deserialize, Serialize};

/// HTTP method for an API endpoint
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
}

/// API endpoint definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEndpoint {
    /// Endpoint path (e.g., "/api/discover-agents")
    pub path: String,
    /// HTTP method
    pub method: HttpMethod,
    /// Handler function name in Rust
    pub handler: String,
    /// Request type name (if any)
    pub request_type: Option<String>,
    /// Response type name
    pub response_type: String,
    /// Description for documentation
    pub description: String,
}

/// Complete API definition
pub struct ApiDefinition;

impl ApiDefinition {
    /// Get all API endpoints
    pub fn endpoints() -> Vec<ApiEndpoint> {
        vec![
            ApiEndpoint {
                path: "/api/health".to_string(),
                method: HttpMethod::GET,
                handler: "health".to_string(),
                request_type: None,
                response_type: "HealthResponse".to_string(),
                description: "Health check endpoint".to_string(),
            },
            ApiEndpoint {
                path: "/api/discover-agents".to_string(),
                method: HttpMethod::GET,
                handler: "discover_agents".to_string(),
                request_type: None,
                response_type: "Vec<DiscoveredHost>".to_string(),
                description: "Discover all available agents on the network".to_string(),
            },
            ApiEndpoint {
                path: "/api/discover-tailscale".to_string(),
                method: HttpMethod::GET,
                handler: "discover_via_tailscale".to_string(),
                request_type: None,
                response_type: "Vec<DiscoveredHost>".to_string(),
                description: "Discover agents via Tailscale".to_string(),
            },
            ApiEndpoint {
                path: "/api/discover-local".to_string(),
                method: HttpMethod::GET,
                handler: "discover_via_local_network".to_string(),
                request_type: None,
                response_type: "Vec<DiscoveredHost>".to_string(),
                description: "Discover agents on local network".to_string(),
            },
            ApiEndpoint {
                path: "/api/ping-agent".to_string(),
                method: HttpMethod::POST,
                handler: "ping_agent".to_string(),
                request_type: Some("PingAgentRequest".to_string()),
                response_type: "bool".to_string(),
                description: "Ping an agent at the given address".to_string(),
            },
            ApiEndpoint {
                path: "/api/host-info".to_string(),
                method: HttpMethod::POST,
                handler: "get_host_info".to_string(),
                request_type: Some("GetHostInfoRequest".to_string()),
                response_type: "HostInfo".to_string(),
                description: "Get host information from an agent".to_string(),
            },
            ApiEndpoint {
                path: "/api/execute-command".to_string(),
                method: HttpMethod::POST,
                handler: "execute_command".to_string(),
                request_type: Some("ExecuteCommandRequest".to_string()),
                response_type: "String".to_string(),
                description: "Execute a command on a remote agent".to_string(),
            },
            ApiEndpoint {
                path: "/api/version".to_string(),
                method: HttpMethod::GET,
                handler: "get_version".to_string(),
                request_type: None,
                response_type: "String".to_string(),
                description: "Get the version of the Halvor client".to_string(),
            },
        ]
    }
}

