mod commands;

use anyhow::Result;
use clap::Parser;
use crate::Commands;

#[derive(Parser)]
#[command(name = "halvor")]
#[command(about = "Halvor - CLI tool for managing homelab infrastructure", long_about = None)]
#[command(version = commands::utils::get_version_string())]
struct Cli {
    /// Hostname to operate on (defaults to localhost if not provided)
    #[arg(long, short = 'H', value_name = "HOSTNAME", global = true)]
    hostname: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

fn main() -> Result<()> {
    // Handle version flags before parsing (to show channel info)
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 2 && (args[1] == "--version" || args[1] == "-V") {
        commands::utils::print_version_with_channel();
        return Ok(());
    }

    // Check for updates (non-blocking, only in production mode)
    commands::utils::check_for_updates();

    let cli = Cli::parse();
    commands::handle_command(cli.hostname, cli.command)?;

    Ok(())
}
