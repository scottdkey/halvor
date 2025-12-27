// CLI types for halvor CLI

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
        /// Helm repository URL for external charts (e.g., https://pkgs.tailscale.com/helmcharts)
        #[arg(long)]
        repo: Option<String>,
        /// Helm repository name (defaults to chart name if not provided)
        #[arg(long)]
        repo_name: Option<String>,
        /// Custom release name for Helm charts (allows multiple instances of the same app, e.g., radarr-4k)
        #[arg(long)]
        name: Option<String>,
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
        command: Option<crate::commands::config::ConfigCommands>,
    },
    /// Database operations (migrations, backup, generate)
    Db {
        #[command(subcommand)]
        command: crate::commands::config::DbCommands,
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
    /// Generate build artifacts (migrations, FFI bindings)
    Generate {
        #[command(subcommand)]
        command: crate::commands::generate::GenerateCommands,
    },
    /// Initialize K3s cluster (primary control plane node) or prepare a node for joining
    Init {
        /// Token for cluster join (generated if not provided)
        #[arg(long)]
        token: Option<String>,
        /// Skip confirmation prompts
        #[arg(long, short = 'y')]
        yes: bool,
        /// Skip K3s initialization - only install tools and configure node (useful for nodes that will join an existing cluster)
        #[arg(long)]
        skip_k3s: bool,
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
    /// Show status of services (mesh overview by default)
    Status {
        #[command(subcommand)]
        command: Option<crate::commands::status::StatusCommands>,
    },
    /// Manage halvor agent (start, stop, discover, sync)
    Agent {
        #[command(subcommand)]
        command: crate::commands::agent::AgentCommands,
    },
    /// Kubernetes context management (switch between direct and tailscale)
    #[command(name = "k8s", alias = "kube")]
    K8s {
        #[command(subcommand)]
        command: crate::commands::k8s::K8sCommands,
    },
}
