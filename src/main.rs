pub mod agent;
mod commands;
pub mod config;
pub mod db;
pub mod services;
pub mod utils;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "halvor")]
#[command(about = "Homelab Automation Layer - CLI tool for managing homelab infrastructure", long_about = None)]
#[command(version = commands::utils::get_version_string())]
struct Cli {
    /// Hostname to operate on (defaults to localhost if not provided)
    #[arg(long, short = 'H', value_name = "HOSTNAME", global = true)]
    hostname: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Backup services, config, and database
    Backup {
        /// Service to backup (e.g., portainer, sonarr). If not provided, interactive selection
        service: Option<String>,
        /// Backup to env location instead of backup path
        #[arg(long)]
        env: bool,
        /// List available backups instead of creating one
        #[arg(long)]
        list: bool,
        /// Backup the database (unencrypted SQLite backup)
        #[arg(long)]
        db: bool,
        /// Path to save database backup (only used with --db)
        #[arg(long)]
        path: Option<String>,
    },
    /// Restore services, config, or database
    Restore {
        /// Service to restore (e.g., portainer, sonarr). If not provided, interactive selection
        service: Option<String>,
        /// Restore from env location instead of backup path
        #[arg(long)]
        env: bool,
        /// Specific backup timestamp to restore (required when service is specified)
        #[arg(long)]
        backup: Option<String>,
    },
    /// Sync encrypted data between hal installations
    Sync {
        /// Pull data from remote instead of pushing
        #[arg(long)]
        pull: bool,
    },
    /// List services or hosts
    List {
        /// Show verbose information
        #[arg(long)]
        verbose: bool,
    },
    /// Install a service on a host
    Install {
        /// Service to install: docker, tailscale, portainer, npm
        service: String,
        /// Portainer edition (ce or be) - only used with portainer
        #[arg(long, default_value = "ce")]
        edition: String,
        /// Install Portainer host (with UI) instead of agent - only used with portainer
        #[arg(long)]
        host: bool,
    },
    /// Uninstall a service from a host or halvor itself
    Uninstall {
        /// Service to uninstall: npm, portainer, smb. If not provided, guided uninstall of halvor
        service: Option<String>,
    },
    /// Provision a host (install Docker, Tailscale, Portainer)
    Provision {
        /// Install Portainer host instead of Portainer Agent
        #[arg(long)]
        portainer_host: bool,
        /// Portainer edition to install (ce or be). Only used with --portainer-host
        #[arg(long, default_value = "ce")]
        portainer_edition: String,
    },
    /// Setup and mount SMB shares
    Smb {
        /// Unmount and remove SMB mounts
        #[arg(long)]
        uninstall: bool,
    },
    /// Diagnose Docker daemon issues
    Docker {
        /// Run diagnostics instead of installing
        #[arg(long)]
        diagnose: bool,
    },
    /// Automatically create proxy hosts in Nginx Proxy Manager
    Npm {
        /// Docker compose file to read services from (e.g., media.docker-compose.yml)
        #[arg(default_value = "")]
        compose_file: String,
        /// Create proxy host for a specific service (e.g., portainer:9000 or npm:81)
        #[arg(long)]
        service: Option<String>,
    },
    /// Build and push VPN container image to GitHub Container Registry
    Vpn {
        #[command(subcommand)]
        command: commands::pia_vpn::VpnCommands,
    },
    /// Configure HAL settings (environment file location, etc.)
    Config {
        /// Show verbose output (including passwords)
        #[arg(short, long)]
        verbose: bool,
        /// Show database configuration instead of .env
        #[arg(long)]
        db: bool,
        #[command(subcommand)]
        command: Option<commands::config::ConfigCommands>,
    },
    /// Database operations (migrations, backup, generate)
    Db {
        #[command(subcommand)]
        command: commands::config::DbCommands,
    },
    /// Check for and install updates
    Update {
        /// Use experimental channel for updates (version less, continuously updated)
        #[arg(long)]
        experimental: bool,
        /// Force download and install the latest version (skips version check)
        #[arg(long)]
        force: bool,
    },
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
