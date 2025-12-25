// Library crate for halvor - exposes modules for use by other crates
pub mod agent;
pub mod commands;
pub mod config;
pub mod db;
#[cfg(feature = "ffi")]
pub mod ffi;
pub mod services;
pub mod utils;

// CLI-specific types (used by both library and binary)
use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
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
    /// Install an app on a host
    Install {
        /// App to install (e.g., docker, sonarr, portainer). Use --list to see all.
        app: Option<String>,
        /// List all available apps
        #[arg(long)]
        list: bool,
    },
    /// Uninstall a service from a host or halvor itself
    Uninstall {
        /// Service to uninstall (e.g., portainer, smb, nginx-proxy-manager). If not provided, guided uninstall of halvor
        service: Option<String>,
    },
    /// Configure halvor settings (environment file location, etc.)
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
    /// Update halvor or installed apps
    Update {
        /// App to update (e.g., docker, tailscale, portainer). If not provided, updates everything on the system.
        app: Option<String>,
        /// Use experimental channel for halvor updates (version less, continuously updated)
        #[arg(long)]
        experimental: bool,
        /// Force download and install the latest version (skips version check)
        #[arg(long)]
        force: bool,
    },
    /// Build applications for different platforms
    Build {
        #[command(subcommand)]
        command: commands::build::BuildCommands,
    },
    /// Development mode for different platforms
    Dev {
        #[command(subcommand)]
        command: commands::dev::DevCommands,
    },
    /// Generate build artifacts (migrations, FFI bindings)
    Generate {
        #[command(subcommand)]
        command: commands::generate::GenerateCommands,
    },
    /// Initialize K3s cluster (primary control plane node)
    Init {
        /// Token for cluster join (generated if not provided)
        #[arg(long)]
        token: Option<String>,
        /// Skip confirmation prompts
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Join a node to the K3s cluster
    Join {
        /// Target hostname to join to the cluster (use -H/--hostname to specify)
        #[arg(value_name = "HOSTNAME")]
        join_hostname: Option<String>,
        /// First control plane node address (e.g., frigg or 192.168.1.10). If not provided, will try to auto-detect from config.
        #[arg(long)]
        server: Option<String>,
        /// Cluster join token (if not provided, will be loaded from K3S_TOKEN env var or fetched from server)
        #[arg(long)]
        token: Option<String>,
        /// Join as control plane node (default: false, use --control-plane to join as control plane)
        #[arg(long, action = clap::ArgAction::SetTrue)]
        control_plane: bool,
    },
    /// Show status of services
    Status {
        #[command(subcommand)]
        command: commands::status::StatusCommands,
    },
    /// Configure Tailscale integration for K3s cluster
    Configure {
        /// Target hostname (default: localhost)
        #[arg(value_name = "HOSTNAME")]
        hostname: Option<String>,
    },
}
