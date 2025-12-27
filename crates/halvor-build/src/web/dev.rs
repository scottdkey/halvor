// Web development modes
use crate::common::execute_command;
use halvor_web;
use anyhow::Result;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::Command;

/// Start web development in bare metal mode (Rust server + Svelte dev, no Docker)
pub async fn dev_web_bare_metal(port: u16, static_dir: Option<PathBuf>) -> Result<()> {
    // Set HALVOR_WEB_DIR if static_dir is provided
    if let Some(dir) = &static_dir {
        unsafe {
            std::env::set_var("HALVOR_WEB_DIR", dir.to_string_lossy().to_string());
        }
    }

    // Start agent with web server on bare metal
    // Agent on default port (13500) or can be configured
    // Web server on the specified port
    let agent_port = 13500; // Default agent port
    println!("🚀 Starting halvor agent and web server locally (bare metal)...");
    println!(
        "🔌 Agent API available on port {} (for CLI connections)",
        agent_port
    );
    println!("🌐 Web UI available at http://localhost:{}", port);

    // Use halvor-web to start the server
    // This will start both the agent and web server
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;
    let static_dir = static_dir.unwrap_or_else(|| PathBuf::from("projects/web/build"));
    halvor_web::start_server(addr, static_dir, Some(agent_port)).await?;

    Ok(())
}

/// Start web development in Docker mode
pub async fn dev_web_docker(_port: u16) -> Result<()> {
    println!("Starting web development in Docker...");

    // Ensure halvor-data directory exists at project root for bind mount
    let data_dir = std::path::Path::new("halvor-data");
    if !data_dir.exists() {
        std::fs::create_dir_all(data_dir)?;
        println!("Created halvor-data directory at project root");
    }

    // Build and start Docker container (only dev service)
    let mut docker_cmd = Command::new("docker-compose");
    docker_cmd
        .arg("up")
        .arg("--build")
        .arg("halvor-web-dev")
        .current_dir("projects/web");

    execute_command(docker_cmd, "Docker dev failed")?;

    Ok(())
}

/// Start web app in production mode (Docker)
pub async fn dev_web_prod() -> Result<()> {
    println!("Starting web app in production mode (Docker)...");
    let web_dir = PathBuf::from("projects/web");
    let mut docker_cmd = Command::new("docker-compose");
    docker_cmd
        .args(["up", "halvor-web-prod"])
        .current_dir(&web_dir);

    execute_command(docker_cmd, "Failed to run web production container")?;

    Ok(())
}

