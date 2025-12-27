use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(name = "halvor-agent")]
#[command(about = "Halvor Agent Server")]
struct Args {
    /// Port for agent API
    #[arg(long, default_value = "13500")]
    port: u16,
    /// Enable web UI on the same port (requires halvor CLI with web UI support)
    #[arg(long)]
    ui: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    println!("Starting Halvor Agent");
    println!("  Agent API: http://0.0.0.0:{}", args.port);
    
    if args.ui {
        println!("  Note: Web UI requires 'halvor agent start --ui' (CLI integration)");
        println!("  This binary only runs the agent server. Use the CLI for web UI.");
    }

    // Just start agent server (web UI integration is handled by CLI)
    halvor_agent::start(args.port, None).await
}
