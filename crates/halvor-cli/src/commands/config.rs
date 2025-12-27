use halvor_core::config;
use halvor_core::config::config_manager;
use halvor_db as db;
use anyhow::{Context, Result};
use std::path::Path;

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
    /// Get kubeconfig for K3s cluster
    Kubeconfig {
        /// Set up local kubectl context (named 'halvor')
        #[arg(long)]
        setup: bool,
        /// Run diagnostics to check API server accessibility
        #[arg(long)]
        diagnose: bool,
        /// Primary control plane hostname (defaults to first k3s node in config)
        #[arg(short = 'H', long)]
        hostname: Option<String>,
    },
    /// Regenerate K3s certificates with Tailscale integration
    Regenerate {
        /// Target hostname (defaults to localhost)
        #[arg(short = 'H', long)]
        hostname: Option<String>,
        /// Skip confirmation prompts
        #[arg(long, short = 'y')]
        yes: bool,
    },
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

#[derive(clap::Subcommand, Clone, Debug)]
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

/// Handle config commands
pub fn handle_config(
    _arg: Option<&str>,
    _verbose: bool,
    _db: bool,
    command: Option<&ConfigCommands>,
) -> Result<()> {
    match command {
        Some(ConfigCommands::List) => {
            let halvor_dir = config::find_halvor_dir()?;
            let env_config = config::load_env_config(&halvor_dir)?;
            println!("Hosts:");
            for (name, host_config) in &env_config.hosts {
                println!("  {}: ip={:?}, hostname={:?}, backup_path={:?}",
                    name,
                    host_config.ip,
                    host_config.hostname,
                    host_config.backup_path
                );
            }
            Ok(())
        }
        Some(ConfigCommands::Init) => {
            config_manager::init_config_interactive()
        }
        Some(ConfigCommands::SetEnv { path }) => {
            config_manager::set_env_file_path(Path::new(path))
        }
        Some(ConfigCommands::SetStable) => {
            let mut hal_config = config_manager::load_config()?;
            hal_config.release_channel = config_manager::ReleaseChannel::Stable;
            config_manager::save_config(&hal_config)?;
            println!("✓ Release channel set to stable");
            Ok(())
        }
        Some(ConfigCommands::SetExperimental) => {
            let mut hal_config = config_manager::load_config()?;
            hal_config.release_channel = config_manager::ReleaseChannel::Experimental;
            config_manager::save_config(&hal_config)?;
            println!("✓ Release channel set to experimental");
            Ok(())
        }
        Some(ConfigCommands::Create { command: _create_cmd }) => {
            anyhow::bail!("Create command not yet fully implemented")
        }
        Some(ConfigCommands::Env) => {
            anyhow::bail!("Env command not yet fully implemented")
        }
        Some(ConfigCommands::SetBackup { hostname }) => {
            anyhow::bail!("SetBackup command not yet fully implemented (hostname: {:?})", hostname)
        }
        Some(ConfigCommands::Commit) => {
            anyhow::bail!("Commit command not yet fully implemented")
        }
        Some(ConfigCommands::Backup) => {
            anyhow::bail!("Backup command not yet fully implemented")
        }
        Some(ConfigCommands::Delete { from_env }) => {
            anyhow::bail!("Delete command not yet fully implemented (from_env: {})", from_env)
        }
        Some(ConfigCommands::Ip { value }) => {
            anyhow::bail!("Ip command not yet fully implemented (value: {})", value)
        }
        Some(ConfigCommands::Hostname { value }) => {
            anyhow::bail!("Hostname command not yet fully implemented (value: {})", value)
        }
        Some(ConfigCommands::BackupPath { value }) => {
            anyhow::bail!("BackupPath command not yet fully implemented (value: {})", value)
        }
        Some(ConfigCommands::Diff) => {
            anyhow::bail!("Diff command not yet fully implemented")
        }
        Some(ConfigCommands::Kubeconfig { setup, diagnose, hostname }) => {
            anyhow::bail!("Kubeconfig command not yet fully implemented (setup: {}, diagnose: {}, hostname: {:?})", setup, diagnose, hostname)
        }
        Some(ConfigCommands::Regenerate { hostname, yes }) => {
            anyhow::bail!("Regenerate command not yet fully implemented (hostname: {:?}, yes: {})", hostname, yes)
        }
        None => {
            // Show config summary
            let hal_config = config_manager::load_config()?;
            println!("Configuration:");
            if let Some(ref env_path) = hal_config.env_file_path {
                println!("  Environment file: {}", env_path.display());
            } else {
                println!("  Environment file: (not set)");
            }
            println!("  Release channel: {:?}", hal_config.release_channel);
            Ok(())
        }
    }
}

/// Handle db subcommands
pub fn handle_db_command(command: DbCommands) -> Result<()> {
    match command {
        DbCommands::Backup { path } => {
            let db_path = db::get_db_path()?;
            if !db_path.exists() {
                anyhow::bail!("Database not found at: {}", db_path.display());
            }
            let backup_path = if let Some(p) = path {
                Path::new(&p).to_path_buf()
            } else {
                use chrono::Utc;
                let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
                std::env::current_dir()?.join(format!("halvor-backup-{}.db", timestamp))
            };
            std::fs::copy(&db_path, &backup_path)
                .with_context(|| format!("Failed to copy database to {}", backup_path.display()))?;
            println!("✓ Database backup created: {}", backup_path.display());
            Ok(())
        }
        DbCommands::Generate => {
            anyhow::bail!("Generate command not yet fully implemented")
        }
        DbCommands::Migrate { command: migrate_cmd } => {
            anyhow::bail!("Migrate command not yet fully implemented (command: {:?})", migrate_cmd)
        }
        DbCommands::Sync => {
            anyhow::bail!("Sync command not yet fully implemented")
        }
        DbCommands::Restore => {
            anyhow::bail!("Restore command not yet fully implemented")
        }
    }
}
