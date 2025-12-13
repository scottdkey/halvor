// Web server for serving Svelte app and exposing FFI functions via HTTP API
use crate::ffi::client::HalvorClient;
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

#[derive(Clone)]
pub struct AppState {
    pub client: Arc<HalvorClient>,
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

// API Handlers
async fn discover_agents(State(state): State<AppState>) -> impl IntoResponse {
    match state.client.discover_agents() {
        Ok(hosts) => (StatusCode::OK, Json(ApiResponse::success(hosts))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<Vec<_>>::error(e)),
        ),
    }
}

async fn discover_via_tailscale(State(state): State<AppState>) -> impl IntoResponse {
    match state.client.discover_via_tailscale() {
        Ok(hosts) => (StatusCode::OK, Json(ApiResponse::success(hosts))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<Vec<_>>::error(e)),
        ),
    }
}

async fn discover_via_local_network(State(state): State<AppState>) -> impl IntoResponse {
    match state.client.discover_via_local_network() {
        Ok(hosts) => (StatusCode::OK, Json(ApiResponse::success(hosts))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<Vec<_>>::error(e)),
        ),
    }
}

async fn ping_agent(
    State(state): State<AppState>,
    Json(req): Json<PingAgentRequest>,
) -> impl IntoResponse {
    match state.client.ping_agent(req.host, req.port) {
        Ok(reachable) => (StatusCode::OK, Json(ApiResponse::success(reachable))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<bool>::error(e)),
        ),
    }
}

async fn get_host_info(
    State(state): State<AppState>,
    Json(req): Json<GetHostInfoRequest>,
) -> impl IntoResponse {
    match state.client.get_host_info(req.host, req.port) {
        Ok(info) => (StatusCode::OK, Json(ApiResponse::success(info))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<crate::agent::server::HostInfo>::error(e)),
        ),
    }
}

async fn execute_command(
    State(state): State<AppState>,
    Json(req): Json<ExecuteCommandRequest>,
) -> impl IntoResponse {
    match state
        .client
        .execute_command(req.host, req.port, req.command, req.args)
    {
        Ok(output) => (StatusCode::OK, Json(ApiResponse::success(output))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<String>::error(e)),
        ),
    }
}

async fn get_version(State(state): State<AppState>) -> impl IntoResponse {
    match state.client.get_version() {
        Ok(version) => (StatusCode::OK, Json(ApiResponse::success(version))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<String>::error(e)),
        ),
    }
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
    let client = Arc::new(HalvorClient::new(agent_port));
    let state = AppState {
        client,
        static_dir: static_dir.clone(),
    };

    // Build the router
    let app = Router::new()
        // API routes
        .route("/api/health", get(health))
        .route("/api/discover-agents", get(discover_agents))
        .route("/api/discover-tailscale", get(discover_via_tailscale))
        .route("/api/discover-local", get(discover_via_local_network))
        .route("/api/ping-agent", post(ping_agent))
        .route("/api/host-info", post(get_host_info))
        .route("/api/execute-command", post(execute_command))
        .route("/api/version", get(get_version))
        // Serve static files (Svelte app)
        .nest_service("/", ServeDir::new(&static_dir))
        .layer(CorsLayer::permissive())
        .with_state(state);

    println!("🚀 Halvor web server starting on http://{}", addr);
    println!("📁 Serving static files from: {}", static_dir.display());
    println!("🔌 API available at http://{}/api/*", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
