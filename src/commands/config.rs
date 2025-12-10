use crate::config::config_manager;
use crate::config::service;
use crate::db;
use anyhow::Result;

#[derive(clap::Subcommand, Clone)]
pub enum ConfigCommands {
    /// Show current configuration
    Show,
    /// Initialize or update HAL configuration (interactive)
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
    /// Backup SQLite database
    Db {
        #[command(subcommand)]
        command: DbCommands,
    },
    /// Set backup location (for current system if no hostname provided)
    Backup {
        /// Hostname to set backup location for (only used when called without hostname)
        hostname: Option<String>,
    },
    /// Commit host configuration to database (from .env to DB)
    Commit,
    /// Write host configuration back to .env file (from DB to .env)
    #[command(name = "backup")]
    BackupToEnv,
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
    /// Set hostname for hostname (primary hostname)
    Hostname {
        /// Hostname value
        value: String,
    },
    /// Set Tailscale hostname (optional, different from primary hostname)
    Tailscale {
        /// Tailscale hostname value
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
    /// Manage database migrations
    Migrate {
        #[command(subcommand)]
        command: MigrateCommands,
    },
}

#[derive(clap::Subcommand, Clone)]
pub enum MigrateCommands {
    /// Run all pending migrations
    All,
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
    arg: Option<&str>,
    verbose: bool,
    db: bool,
    command: Option<&ConfigCommands>,
) -> Result<()> {
    // Known global commands that should not be treated as hostnames
    let global_commands = [
        "show",
        "init",
        "set-env",
        "stable",
        "experimental",
        "create",
        "env",
        "db",
        "backup",
        "commit",
        "backup-to-env",
        "delete",
        "diff",
    ];

    // If arg is provided and it's not a known command, treat it as a hostname
    if let Some(arg_str) = arg {
        if !global_commands.contains(&arg_str.to_lowercase().as_str()) {
            // This is a hostname
            let hostname = arg_str;
            match command {
                None | Some(ConfigCommands::Show) => {
                    service::show_host_config(hostname)?;
                }
                Some(ConfigCommands::Commit) => {
                    service::commit_host_config_to_db(hostname)?;
                }
                Some(ConfigCommands::BackupToEnv) => {
                    service::backup_host_config_to_env(hostname)?;
                }
                Some(ConfigCommands::Delete { from_env }) => {
                    service::delete_host_config(hostname, *from_env)?;
                }
                Some(ConfigCommands::Ip { value }) => {
                    service::set_host_field(hostname, "ip", &value)?;
                }
                Some(ConfigCommands::Hostname { value }) => {
                    service::set_host_field(hostname, "hostname", &value)?;
                }
                Some(ConfigCommands::Tailscale { value }) => {
                    service::set_host_field(hostname, "tailscale", &value)?;
                }
                Some(ConfigCommands::BackupPath { value }) => {
                    service::set_host_field(hostname, "backup_path", &value)?;
                }
                Some(ConfigCommands::Backup { hostname: _ }) => {
                    // This shouldn't happen when hostname is provided, but handle it
                    service::set_backup_location(Some(hostname))?;
                }
                Some(ConfigCommands::Diff) => {
                    anyhow::bail!(
                        "Diff command is global only. Use 'halvor config diff' to see all differences"
                    );
                }
                _ => {
                    anyhow::bail!("Command not valid for hostname-specific operations");
                }
            }
            return Ok(());
        }
    }

    // Handle global config commands
    // If arg is a known command, use it; otherwise use the subcommand
    let cmd = if let Some(arg_str) = arg {
        // Map string to command
        match arg_str.to_lowercase().as_str() {
            "show" => ConfigCommands::Show,
            "init" => ConfigCommands::Init,
            "env" => ConfigCommands::Env,
            "stable" => ConfigCommands::SetStable,
            "experimental" => ConfigCommands::SetExperimental,
            "commit" => ConfigCommands::Commit,
            "backup" => ConfigCommands::BackupToEnv,
            "diff" => ConfigCommands::Diff,
            _ => {
                // Use the subcommand if provided, otherwise default to Show
                command.cloned().unwrap_or(ConfigCommands::Show)
            }
        }
    } else {
        // Use the subcommand if provided, otherwise default to Show
        command.cloned().unwrap_or(ConfigCommands::Show)
    };

    match cmd {
        ConfigCommands::Show => {
            if db {
                service::show_db_config(verbose)?;
            } else {
                service::show_current_config(verbose)?;
            }
        }
        ConfigCommands::Commit => {
            service::commit_all_to_db()?;
        }
        ConfigCommands::BackupToEnv => {
            service::backup_all_to_env()?;
        }
        ConfigCommands::Init => {
            config_manager::init_config_interactive()?;
        }
        ConfigCommands::SetEnv { path } => {
            service::set_env_path(path.as_str())?;
        }
        ConfigCommands::SetStable => {
            config_manager::set_release_channel(config_manager::ReleaseChannel::Stable)?;
        }
        ConfigCommands::SetExperimental => {
            config_manager::set_release_channel(config_manager::ReleaseChannel::Experimental)?;
        }
        ConfigCommands::Create { command } => {
            handle_create_config(command)?;
        }
        ConfigCommands::Env => {
            service::create_example_env_file()?;
        }
        ConfigCommands::Db { command } => {
            handle_db_command(command)?;
        }
        ConfigCommands::Backup { hostname } => {
            service::set_backup_location(hostname.as_deref())?;
        }
        ConfigCommands::Delete { .. } => {
            anyhow::bail!(
                "Delete requires a hostname. Usage: halvor config <hostname> delete [--from-env]"
            );
        }
        ConfigCommands::Diff => {
            service::show_config_diff()?;
        }
        ConfigCommands::Ip { .. }
        | ConfigCommands::Hostname { .. }
        | ConfigCommands::Tailscale { .. }
        | ConfigCommands::BackupPath { .. }
        | ConfigCommands::Commit
        | ConfigCommands::BackupToEnv => {
            anyhow::bail!(
                "This command requires a hostname. Usage: halvor config <hostname> <command>"
            );
        }
    }

    Ok(())
}

/// Handle db subcommands
fn handle_db_command(command: DbCommands) -> Result<()> {
    match command {
        DbCommands::Generate => {
            db::core::generator::generate_structs()?;
        }
        DbCommands::Backup { path } => {
            service::backup_database(path.as_deref())?;
        }
        DbCommands::Migrate { command } => {
            handle_migrate(command)?;
        }
    }
    Ok(())
}

/// Handle migrate commands - calls db::migrate functions
fn handle_migrate(command: MigrateCommands) -> Result<()> {
    match command {
        MigrateCommands::All => {
            db::migrate::migrate_all()?;
        }
        MigrateCommands::Up => {
            db::migrate::migrate_up()?;
        }
        MigrateCommands::Down => {
            db::migrate::migrate_down()?;
        }
        MigrateCommands::List => {
            db::migrate::migrate_list()?;
        }
        MigrateCommands::Generate { description }
        | MigrateCommands::GenerateShort { description } => {
            db::migrate::generate_migration(description)?;
        }
    }
    Ok(())
}

fn handle_create_config(command: CreateConfigCommands) -> Result<()> {
    match command {
        CreateConfigCommands::App => {
            println!(
                "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            );
            println!("Create App Configuration");
            println!(
                "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            );
            println!();
            println!("⚠ App configuration creation not yet implemented");
        }
        CreateConfigCommands::Smb { server_name: _ } => {
            println!(
                "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            );
            println!("Create SMB Server Configuration");
            println!(
                "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            );
            println!();
            println!("⚠ SMB configuration creation not yet implemented");
        }
        CreateConfigCommands::Ssh { hostname: _ } => {
            println!(
                "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            );
            println!("Create SSH Host Configuration");
            println!(
                "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            );
            println!();
            println!("⚠ SSH configuration creation not yet implemented");
        }
    }
    Ok(())
}
