mod backup;
mod config;
mod config_manager;
mod crypto;
mod db;
mod docker;
mod exec;
mod networking;
mod npm;
mod portainer;
mod provision;
mod smb;
mod ssh;
mod sync;
mod tailscale;
mod update;
mod vpn;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::env;

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
#[command(version = get_version_string())]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

fn get_version_string() -> &'static str {
    // Return base version - we'll enhance it in print_version_with_channel
    env!("CARGO_PKG_VERSION")
}

fn print_version_with_channel() {
    let version = env!("CARGO_PKG_VERSION");

    // Try to determine if this is an experimental build
    // by checking if the executable timestamp matches an experimental release
    let version_string = if let Ok(channel) = update::detect_release_channel() {
        match channel {
            update::ReleaseChannel::Experimental => format!("{} (experimental)", version),
            update::ReleaseChannel::Stable => format!("{} (stable)", version),
            update::ReleaseChannel::Unknown => version.to_string(),
        }
    } else {
        version.to_string()
    };

    println!("hal {}", version_string);
}

#[derive(Subcommand)]
enum Commands {
    /// Install Tailscale on the system
    Tailscale {
        #[command(subcommand)]
        command: TailscaleCommands,
    },
    /// Install Docker on the system
    Docker {
        #[command(subcommand)]
        command: DockerCommands,
    },
    /// Install Portainer on the system
    Portainer {
        #[command(subcommand)]
        command: PortainerCommands,
    },
    /// Provision a remote host (install Docker, Tailscale, Portainer)
    Provision {
        /// Hostname to provision
        hostname: String,
        /// Install Portainer host instead of Portainer Agent
        #[arg(long)]
        portainer_host: bool,
        /// Portainer edition to install (ce or be). Only used with --portainer-host
        #[arg(long, default_value = "ce")]
        portainer_edition: String,
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
    /// Sync encrypted data between hal installations
    Sync {
        /// Hostname to sync with
        hostname: String,
        /// Pull data from remote instead of pushing
        #[arg(long)]
        pull: bool,
    },
    /// Check for and install updates
    Update {
        /// Use experimental channel for updates (versionless, continuously updated)
        #[arg(long)]
        experimental: bool,
        /// Force download and install the latest version (skips version check)
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum TailscaleCommands {
    /// Install Tailscale
    Install {
        /// Hostname to install Tailscale on (defaults to localhost)
        #[arg(default_value = "localhost")]
        hostname: String,
    },
}

#[derive(Subcommand)]
enum DockerCommands {
    /// Install Docker
    Install {
        /// Hostname to install Docker on (defaults to localhost)
        #[arg(default_value = "localhost")]
        hostname: String,
    },
}

#[derive(Subcommand)]
enum PortainerCommands {
    /// Install Portainer (host or agent)
    Install {
        /// Hostname to install Portainer on (defaults to localhost)
        #[arg(default_value = "localhost")]
        hostname: String,
        /// Portainer edition to install (ce or be)
        #[arg(long, default_value = "ce")]
        edition: String,
        /// Install Portainer host (with UI) instead of agent
        #[arg(long)]
        host: bool,
    },
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
    /// Verify VPN is working correctly
    Verify {
        /// Hostname where VPN is running
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
    /// Set release channel to stable
    #[command(name = "stable")]
    SetStable,
    /// Set release channel to experimental
    #[command(name = "experimental")]
    SetExperimental,
}

fn main() -> Result<()> {
    // Handle version flags before parsing (to show channel info)
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 2 && (args[1] == "--version" || args[1] == "-V") {
        print_version_with_channel();
        return Ok(());
    }

    // Check for updates (non-blocking, only in production mode)
    check_for_updates();

    let cli = Cli::parse();

    match cli.command {
        Commands::Tailscale { command } => match command {
            TailscaleCommands::Install { hostname } => {
                if hostname == "localhost" {
                    // For localhost, use the original function (no config needed)
                    tailscale::install_tailscale()?;
                } else {
                    let homelab_dir = config::find_homelab_dir()?;
                    let config = config::load_env_config(&homelab_dir)?;
                    tailscale::install_tailscale_on_host(&hostname, &config)?;
                }
            }
        },
        Commands::Docker { command } => match command {
            DockerCommands::Install { hostname } => {
                let homelab_dir = config::find_homelab_dir()?;
                let config = config::load_env_config(&homelab_dir)?;
                docker::install_docker(&hostname, &config)?;
            }
        },
        Commands::Portainer { command } => match command {
            PortainerCommands::Install {
                hostname,
                edition,
                host,
            } => {
                let homelab_dir = config::find_homelab_dir()?;
                let config = config::load_env_config(&homelab_dir)?;
                if host {
                    portainer::install_portainer_host(&hostname, &edition, &config)?;
                } else {
                    portainer::install_portainer_agent(&hostname, &edition, &config)?;
                }
            }
        },
        Commands::Provision {
            hostname,
            portainer_host,
            portainer_edition,
        } => {
            let homelab_dir = config::find_homelab_dir()?;
            let config = config::load_env_config(&homelab_dir)?;
            provision::provision_host(&hostname, portainer_host, &portainer_edition, &config)?;
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
                let config_dir = config::find_homelab_dir()?;
                let config = config::load_env_config(&config_dir)?;
                // For build, use localhost as default hostname (builds are typically local)
                let build_hostname = "localhost";
                vpn::build_and_push_vpn_image(
                    build_hostname,
                    &github_user,
                    tag.as_deref(),
                    &config,
                )?;
            }
            VpnCommands::Deploy { hostname } => {
                let config_dir = config::find_homelab_dir()?;
                let config = config::load_env_config(&config_dir)?;
                vpn::deploy_vpn(&hostname, &config)?;
            }
            VpnCommands::Verify { hostname } => {
                let config_dir = config::find_homelab_dir()?;
                let config = config::load_env_config(&config_dir)?;
                vpn::verify_vpn(&hostname, &config)?;
            }
        },
        Commands::Update {
            experimental,
            force,
        } => {
            let current_version = env!("CARGO_PKG_VERSION");

            if force {
                // Force mode: get the latest version and install it regardless of current version
                if experimental {
                    println!("Force mode: Downloading latest experimental version...");
                    let latest_version = update::get_latest_experimental_version()?;
                    println!("Latest experimental version: {}", latest_version);
                    update::download_and_install_update(&latest_version)?;
                } else {
                    println!("Force mode: Downloading latest stable version...");
                    let latest_version = update::get_latest_version()?;
                    println!("Latest version: {}", latest_version);
                    update::download_and_install_update(&latest_version)?;
                }
            } else if experimental {
                // Experimental channel: check for updates based on timestamps (versionless)
                if let Ok(Some(new_version)) =
                    update::check_for_experimental_updates(current_version)
                {
                    if update::prompt_for_update(&new_version, current_version)? {
                        update::download_and_install_update(&new_version)?;
                    }
                } else {
                    println!("You're already running the latest experimental version.");
                }
            } else if let Ok(Some(new_version)) = update::check_for_updates(current_version) {
                if update::prompt_for_update(&new_version, current_version)? {
                    update::download_and_install_update(&new_version)?;
                }
            } else {
                println!(
                    "You're already running the latest version: {}",
                    current_version
                );
            }
        }
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
                let channel_name = match hal_config.release_channel {
                    config_manager::ReleaseChannel::Stable => "stable",
                    config_manager::ReleaseChannel::Experimental => "experimental",
                };
                println!("Release channel: {}", channel_name);
                println!();
                if let Ok(db_path) = db::get_db_path() {
                    println!("Database location: {}", db_path.display());
                }
            }
            ConfigCommands::SetStable => {
                config_manager::set_release_channel(config_manager::ReleaseChannel::Stable)?;
            }
            ConfigCommands::SetExperimental => {
                config_manager::set_release_channel(config_manager::ReleaseChannel::Experimental)?;
            }
        },
        Commands::Sync { hostname, pull } => {
            let homelab_dir = config::find_homelab_dir()?;
            let config = config::load_env_config(&homelab_dir)?;
            sync::sync_data(&hostname, pull, &config)?;
        }
    }

    Ok(())
}
