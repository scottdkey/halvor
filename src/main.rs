mod backup;
mod config;
mod npm;
mod provision;
mod smb;
mod ssh;
mod tailscale;
mod vpn;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "hal")]
#[command(about = "Homelab Automation Layer - CLI tool for managing homelab infrastructure", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Connect to a host via SSH (tries local IP, then Tailscale)
    Ssh {
        /// Hostname to connect to
        hostname: String,
        /// Username for SSH connection (if not provided, will prompt or use default)
        #[arg(long, short = 'u')]
        user: Option<String>,
        /// Remove offending host keys from known_hosts before connecting
        #[arg(long, short = 'f')]
        fix_keys: bool,
        /// Copy SSH public key to remote host for passwordless authentication
        #[arg(long)]
        keys: bool,
        /// Additional SSH arguments
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        ssh_args: Vec<String>,
    },
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

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Ssh {
            hostname,
            user,
            fix_keys,
            keys,
            ssh_args,
        } => {
            let homelab_dir = config::find_homelab_dir()?;
            let config = config::load_env_config(&homelab_dir)?;
            ssh::ssh_to_host(&hostname, user, fix_keys, keys, &ssh_args, &config)?;
        }
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
                let homelab_dir = config::find_homelab_dir()?;
                let config = config::load_env_config(&homelab_dir)?;
                vpn::deploy_vpn(&hostname, &config)?;
            }
        },
    }

    Ok(())
}
