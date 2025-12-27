use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(name = "halvor-agent")]
#[command(about = "Halvor Agent Server")]
struct Args {
    /// Port for agent API
    #[arg(long, default_value = "13500")]
    port: u16,

    /// Port for web UI (enables UI if provided)
    #[arg(long)]
    web_port: Option<u16>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    println!("Starting Halvor Agent");
    println!("  Agent API: http://0.0.0.0:{}", args.port);
    if let Some(web_port) = args.web_port {
        println!("  Web UI: http://0.0.0.0:{}", web_port);
    }

    halvor_agent::start(args.port, args.web_port).await
}
