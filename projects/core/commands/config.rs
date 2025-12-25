use crate::config::service;
use anyhow::Result;

#[derive(clap::Subcommand, Clone)]
pub enum ConfigCommands {
    /// List current configuration
    List,
    /// Initialize or update halvor configuration (interactive)
    Init,
    /// Set the environment file path
    SetEnv {
        /// Path to the .env file
        path: String,
    },
    /// Set release channel to stable
    #[command(name = "stable")]
    SetStable,
    /// Set release channel to experimental
    #[command(name = "experimental")]
    SetExperimental,
    /// Create new configuration
    Create {
        #[command(subcommand)]
        command: CreateConfigCommands,
    },
    /// Create example .env file
    Env,
    /// Set backup location (for current system if no hostname provided)
    SetBackup {
        /// Hostname to set backup location for (only used when called without hostname)
        hostname: Option<String>,
    },
    /// Commit host configuration to database (from .env to DB)
    Commit,
    /// Write host configuration back to .env file (from DB to .env, backs up current .env first)
    #[command(name = "backup")]
    Backup,
    /// Delete host configuration
    Delete {
        /// Also delete from .env file
        #[arg(long)]
        from_env: bool,
    },
    /// Set IP address for hostname
    Ip {
        /// IP address
        value: String,
    },
    /// Set hostname (typically Tailscale hostname)
    Hostname {
        /// Hostname value
        value: String,
    },
    /// Set backup path for hostname
    BackupPath {
        /// Backup path
        value: String,
    },
    /// Show differences between .env and database configurations
    Diff,
}

#[derive(clap::Subcommand, Clone)]
pub enum CreateConfigCommands {
    /// Create app configuration (backup location, etc.)
    App,
    /// Create SMB server configuration
    Smb {
        /// Server name
        server_name: Option<String>,
    },
    /// Create SSH host configuration
    Ssh {
        /// Hostname
        hostname: Option<String>,
    },
}

#[derive(clap::Subcommand, Clone)]
pub enum DbCommands {
    /// Backup the SQLite database
    Backup {
        /// Path to save backup (defaults to current directory with timestamp)
        #[arg(long)]
        path: Option<String>,
    },
    /// Generate Rust structs from database schema
    Generate,
    /// Manage database migrations (defaults to running all pending migrations)
    Migrate {
        #[command(subcommand)]
        command: Option<MigrateCommands>,
    },
    /// Sync environment file to database (load env values into DB, delete DB values not in env)
    Sync,
    /// Restore database from backup
    Restore,
}

#[derive(clap::Subcommand, Clone)]
pub enum MigrateCommands {
    /// Run the next pending migration (migrate forward one)
    Up,
    /// Rollback the last applied migration (migrate backward one)
    Down,
    /// List migrations and interactively select one to migrate to
    List,
    /// Generate a new migration file
    Generate {
        /// Migration description (e.g., "add users table")
        description: Vec<String>,
    },
    /// Alias for generate
    #[command(name = "g")]
    GenerateShort {
        /// Migration description (e.g., "add users table")
        description: Vec<String>,
    },
}

/// Handle config commands - delegates to service layer
pub fn handle_config(
    arg: Option<&str>,
    verbose: bool,
    db: bool,
    command: Option<&ConfigCommands>,
) -> Result<()> {
    service::handle_config_command(arg, verbose, db, command)
}

/// Handle db subcommands - delegates to service layer
pub fn handle_db_command(command: DbCommands) -> Result<()> {
    service::handle_db_command(command)
}
