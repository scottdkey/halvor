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
        /// Install as Helm chart (for Kubernetes)
        #[arg(long)]
        helm: bool,
    },
    /// Uninstall a service from a host or halvor itself
    Uninstall {
        /// Service to uninstall: npm, portainer, smb. If not provided, guided uninstall of halvor
        service: Option<String>,
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
    /// Manage halvor agent daemon (start/stop/status/discover)
    Agent {
        #[command(subcommand)]
        command: commands::agent::AgentCommands,
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
    /// Manage K3s cluster (init, join, status, snapshot)
    K3s {
        #[command(subcommand)]
        command: commands::k3s::K3sCommands,
    },
    /// Deploy and manage Helm charts
    Helm {
        #[command(subcommand)]
        command: commands::helm::HelmCommands,
    },
}
