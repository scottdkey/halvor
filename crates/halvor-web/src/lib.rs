//! Halvor Web Service
//! Web server for serving SolidJS app and exposing FFI functions via HTTP API
//! This is the agent UI - all FFI functions are exposed as REST endpoints
//! Routes are generated from API definitions - see api_def.rs

#[cfg(feature = "agent")]
use halvor_agent::HalvorClient;
use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::{cors::CorsLayer, services::ServeDir};

pub mod api_def;
pub mod client_gen;

#[derive(Clone)]
pub struct AppState {
    #[cfg(feature = "agent")]
    pub client: Arc<HalvorClient>,
    #[cfg(not(feature = "agent"))]
    pub client: Arc<()>, // Placeholder when agent feature is disabled
    pub static_dir: PathBuf,
}

// API Request/Response types
#[derive(Deserialize)]
pub struct PingAgentRequest {
    pub host: String,
    pub port: u16,
}

#[derive(Deserialize)]
pub struct GetHostInfoRequest {
    pub host: String,
    pub port: u16,
}

#[derive(Deserialize)]
pub struct ExecuteCommandRequest {
    pub host: String,
    pub port: u16,
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    fn error(message: String) -> ApiResponse<T> {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

// Helper to convert Result<T, String> to ApiResponse
fn result_to_response<T>(result: Result<T, String>) -> ApiResponse<T> {
    match result {
        Ok(data) => ApiResponse::success(data),
        Err(err) => ApiResponse::error(err),
    }
}

// API Handlers - All FFI functions exposed as HTTP endpoints

/// Discover all available agents on the network
/// GET /api/discover-agents
#[cfg(feature = "agent")]
async fn discover_agents(State(state): State<AppState>) -> impl IntoResponse {
    let result = state.client.discover_agents();
    let response = result_to_response(result);
    let status = if response.success {
        StatusCode::OK
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    };
    (status, Json(response))
}

/// Discover agents via Tailscale
/// GET /api/discover-tailscale
#[cfg(feature = "agent")]
async fn discover_via_tailscale(State(state): State<AppState>) -> impl IntoResponse {
    let result = state.client.discover_via_tailscale();
    let response = result_to_response(result);
    let status = if response.success {
        StatusCode::OK
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    };
    (status, Json(response))
}

/// Discover agents on local network
/// GET /api/discover-local
#[cfg(feature = "agent")]
async fn discover_via_local_network(State(state): State<AppState>) -> impl IntoResponse {
    let result = state.client.discover_via_local_network();
    let response = result_to_response(result);
    let status = if response.success {
        StatusCode::OK
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    };
    (status, Json(response))
}

/// Ping an agent at the given address
/// POST /api/ping-agent
/// Body: { "host": "hostname", "port": 13500 }
#[cfg(feature = "agent")]
async fn ping_agent(
    State(state): State<AppState>,
    Json(req): Json<PingAgentRequest>,
) -> impl IntoResponse {
    let result = state.client.ping_agent(req.host, req.port);
    let response = result_to_response(result);
    let status = if response.success {
        StatusCode::OK
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    };
    (status, Json(response))
}

/// Get host information from an agent
/// POST /api/host-info
/// Body: { "host": "hostname", "port": 13500 }
#[cfg(feature = "agent")]
async fn get_host_info(
    State(state): State<AppState>,
    Json(req): Json<GetHostInfoRequest>,
) -> impl IntoResponse {
    let result = state.client.get_host_info(req.host, req.port);
    let response = result_to_response(result);
    let status = if response.success {
        StatusCode::OK
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    };
    (status, Json(response))
}

/// Execute a command on a remote agent
/// POST /api/execute-command
/// Body: { "host": "hostname", "port": 13500, "command": "ls", "args": ["-la"] }
#[cfg(feature = "agent")]
async fn execute_command(
    State(state): State<AppState>,
    Json(req): Json<ExecuteCommandRequest>,
) -> impl IntoResponse {
    let result = state
        .client
        .execute_command(req.host, req.port, req.command, req.args);
    let response = result_to_response(result);
    let status = if response.success {
        StatusCode::OK
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    };
    (status, Json(response))
}

/// Get the version of the Halvor client
/// GET /api/version
#[cfg(feature = "agent")]
async fn get_version(State(state): State<AppState>) -> impl IntoResponse {
    let result = state.client.get_version();
    let response = result_to_response(result);
    let status = if response.success {
        StatusCode::OK
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    };
    (status, Json(response))
}

// Health check endpoint
async fn health() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"status": "ok"})))
}

/// Start the web server
pub async fn start_server(
    addr: SocketAddr,
    static_dir: PathBuf,
    agent_port: Option<u16>,
) -> anyhow::Result<()> {
    #[cfg(feature = "agent")]
    let client = Arc::new(HalvorClient::new(agent_port));
    #[cfg(not(feature = "agent"))]
    let client = Arc::new(());
    let state = AppState {
        client,
        static_dir: static_dir.clone(),
    };

    // Build the router from API definitions
    let mut app = Router::new()
        // Health check endpoint
        .route("/api/health", get(health));
    
    #[cfg(feature = "agent")]
    {
        // All FFI functions exposed as HTTP endpoints for SolidJS UI
        // Routes are generated from api_def::ApiDefinition::endpoints()
        use api_def::ApiDefinition;
        use api_def::HttpMethod;
        
        for endpoint in ApiDefinition::endpoints() {
            // Skip health endpoint (already added)
            if endpoint.path == "/api/health" {
                continue;
            }
            
            let route = match endpoint.method {
                HttpMethod::GET => {
                    match endpoint.handler.as_str() {
                        "discover_agents" => get(discover_agents),
                        "discover_via_tailscale" => get(discover_via_tailscale),
                        "discover_via_local_network" => get(discover_via_local_network),
                        "get_version" => get(get_version),
                        _ => continue, // Unknown handler
                    }
                }
                HttpMethod::POST => {
                    match endpoint.handler.as_str() {
                        "ping_agent" => post(ping_agent),
                        "get_host_info" => post(get_host_info),
                        "execute_command" => post(execute_command),
                        _ => continue, // Unknown handler
                    }
                }
                _ => continue, // Unsupported method
            };
            
            app = app.route(&endpoint.path, route);
        }
    }
    
    let app = app
        // Serve static files (SolidJS app)
        .nest_service("/", ServeDir::new(&static_dir))
        .layer(CorsLayer::permissive())
        .with_state(state);

    println!("🚀 Halvor Agent UI server starting on http://{}", addr);
    println!("📁 Serving SolidJS app from: {}", static_dir.display());
    println!("🔌 API available at http://{}/api/*", addr);
    println!("📱 This is the agent UI - iOS and Android will also serve as UIs for the agent");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

