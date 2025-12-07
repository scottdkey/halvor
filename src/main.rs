mod backup;
mod config;
mod config_manager;
mod exec;
mod npm;
mod provision;
mod smb;
mod tailscale;
mod update;
mod vpn;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

fn check_for_updates() {
    // Check for updates in background (non-blocking)
    if let Ok(Some(new_version)) = update::check_for_updates(env!("CARGO_PKG_VERSION")) {
        if let Ok(true) = update::prompt_for_update(&new_version, env!("CARGO_PKG_VERSION")) {
            if let Err(e) = update::download_and_install_update(&new_version) {
                eprintln!("Failed to install update: {}", e);
            }
        }
    }
}

#[derive(Parser)]
#[command(name = "hal")]
#[command(about = "Homelab Automation Layer - CLI tool for managing homelab infrastructure", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Install Tailscale on the system
    Tailscale {
        #[command(subcommand)]
        command: TailscaleCommands,
    },
    /// Provision a remote host (install Docker, Tailscale, Portainer)
    Provision {
        /// Hostname to provision
        hostname: String,
        /// Install Portainer CE instead of Portainer Agent
        #[arg(long)]
        portainer_host: bool,
    },
    /// Setup and mount SMB shares on a remote host
    Smb {
        /// Hostname to setup SMB mounts on
        hostname: String,
        /// Unmount and remove SMB mounts
        #[arg(long)]
        uninstall: bool,
    },
    /// Backup and restore Docker Compose data
    Backup {
        /// Hostname to backup/restore on
        hostname: String,
        #[command(subcommand)]
        command: BackupCommands,
    },
    /// Automatically create proxy hosts in Nginx Proxy Manager from compose file
    Npm {
        /// Hostname where Nginx Proxy Manager is running
        hostname: String,
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
        command: VpnCommands,
    },
    /// Configure HAL settings (environment file location, etc.)
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
}

#[derive(Subcommand)]
enum TailscaleCommands {
    /// Install Tailscale
    Install,
}

#[derive(Subcommand)]
enum VpnCommands {
    /// Build and push VPN container image to GitHub Container Registry
    Build {
        /// GitHub username or organization
        #[arg(long)]
        github_user: String,
        /// Image tag (if not provided, pushes both 'latest' and git hash)
        #[arg(long)]
        tag: Option<String>,
    },
    /// Deploy VPN to a remote host (injects PIA credentials from local .env)
    Deploy {
        /// Hostname to deploy VPN to
        hostname: String,
    },
}

#[derive(Subcommand)]
enum BackupCommands {
    /// Create a backup of all Docker volumes on the host
    Create,
    /// List available backups
    List,
    /// Restore from a backup
    Restore {
        /// Optional: specific backup name (timestamp). If not provided, lists available backups
        #[arg(short, long)]
        backup: Option<String>,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Initialize or update HAL configuration (interactive)
    Init,
    /// Set the environment file path
    SetEnv {
        /// Path to the .env file
        path: String,
    },
    /// Show current configuration
    Show,
}

fn main() -> Result<()> {
    // Check for updates (non-blocking, only in production mode)
    check_for_updates();

    let cli = Cli::parse();

    match cli.command {
        Commands::Tailscale { command } => match command {
            TailscaleCommands::Install => tailscale::install_tailscale()?,
        },
        Commands::Provision {
            hostname,
            portainer_host,
        } => {
            let homelab_dir = config::find_homelab_dir()?;
            let config = config::load_env_config(&homelab_dir)?;
            provision::provision_host(&hostname, portainer_host, &config)?;
        }
        Commands::Smb {
            hostname,
            uninstall,
        } => {
            let homelab_dir = config::find_homelab_dir()?;
            let config = config::load_env_config(&homelab_dir)?;
            if uninstall {
                smb::uninstall_smb_mounts(&hostname, &config)?;
            } else {
                smb::setup_smb_mounts(&hostname, &config)?;
            }
        }
        Commands::Backup { hostname, command } => {
            let homelab_dir = config::find_homelab_dir()?;
            let config = config::load_env_config(&homelab_dir)?;
            match command {
                BackupCommands::Create => backup::backup_host(&hostname, &config)?,
                BackupCommands::List => backup::list_backups(&hostname, &config)?,
                BackupCommands::Restore { backup } => {
                    backup::restore_host(&hostname, backup.as_deref(), &config)?
                }
            }
        }
        Commands::Npm {
            hostname,
            compose_file,
            service,
        } => {
            let homelab_dir = config::find_homelab_dir()?;
            let config = config::load_env_config(&homelab_dir)?;
            // Use tokio runtime for async
            let rt = tokio::runtime::Runtime::new()?;
            if let Some(service_spec) = service {
                rt.block_on(npm::setup_single_proxy_host(
                    &hostname,
                    &service_spec,
                    &config,
                ))?;
            } else if !compose_file.is_empty() {
                rt.block_on(npm::setup_proxy_hosts(&hostname, &compose_file, &config))?;
            } else {
                anyhow::bail!("Either --service or compose_file must be provided");
            }
        }
        Commands::Vpn { command } => match command {
            VpnCommands::Build { github_user, tag } => {
                vpn::build_and_push_vpn_image(&github_user, tag.as_deref())?;
            }
            VpnCommands::Deploy { hostname } => {
                let config_dir = config::find_homelab_dir()?;
                let config = config::load_env_config(&config_dir)?;
                vpn::deploy_vpn(&hostname, &config)?;
            }
        },
        Commands::Config { command } => match command {
            ConfigCommands::Init => {
                config_manager::init_config_interactive()?;
            }
            ConfigCommands::SetEnv { path } => {
                let env_path = std::path::PathBuf::from(&path);
                let env_path = if env_path.is_relative() {
                    std::env::current_dir()?.join(&env_path)
                } else {
                    env_path
                };
                let env_path = env_path
                    .canonicalize()
                    .with_context(|| format!("Failed to resolve path: {}", path))?;

                if !env_path.exists() {
                    anyhow::bail!("File does not exist: {}", env_path.display());
                }

                config_manager::set_env_file_path(&env_path)?;
            }
            ConfigCommands::Show => {
                let hal_config = config_manager::load_config()?;
                println!("HAL Configuration");
                println!("=================");
                println!();

                if let Some(ref env_path) = hal_config.env_file_path {
                    println!("Environment file: {}", env_path.display());
                    if env_path.exists() {
                        println!("  Status: ✓ Found");
                    } else {
                        println!("  Status: ✗ Not found");
                    }
                } else {
                    println!("Environment file: Not configured");
                    println!("  Run 'hal config init' to set it up");
                }

                println!();
                println!(
                    "Config file location: {}",
                    config_manager::get_config_file_path()?.display()
                );
            }
        },
    }

    Ok(())
}
