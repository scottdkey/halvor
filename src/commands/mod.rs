// Command module routing
//
// To add a new command:
// 1. Create a new file in this directory (e.g., `mycommand.rs`)
// 2. Add `pub mod mycommand;` below
// 3. Add the match arm in `handle_command` function

// Declare all command modules - add new modules here
pub mod backup;
pub mod config;
pub mod docker;
pub mod install;
pub mod list;
pub mod npm;
pub mod pia_vpn;
pub mod portainer;
pub mod provision;
pub mod smb;
pub mod sync;
pub mod tailscale;
pub mod uninstall;
pub mod update;
pub mod utils;

use crate::Commands;
use crate::Commands::*;
use anyhow::Result;

/// Dispatch command to appropriate handler
///
/// Routes commands to their respective handlers based on the Commands enum.
/// Each command variant should have a corresponding handler function in its module.
pub fn handle_command(hostname: Option<String>, command: Commands) -> Result<()> {
    match command {
        Backup {
            service,
            env,
            list,
            db,
            path,
        } => {
            if db {
                backup::handle_backup_db(path.as_deref())?;
            } else {
                backup::handle_backup(hostname.as_deref(), service.as_deref(), env, list)?;
            }
        }
        Restore {
            service,
            env,
            backup,
        } => {
            backup::handle_restore(
                hostname.as_deref(),
                service.as_deref(),
                env,
                backup.as_deref(),
            )?;
        }
        Sync { pull } => {
            sync::handle_sync(hostname.as_deref(), pull)?;
        }
        List { verbose } => {
            list::handle_list(hostname.as_deref(), verbose)?;
        }
        Install {
            service,
            edition,
            host,
        } => {
            install::handle_install(hostname.as_deref(), &service, &edition, host)?;
        }
        Uninstall { service } => {
            if let Some(service) = service {
                uninstall::handle_uninstall(hostname.as_deref(), &service)?;
            } else {
                uninstall::handle_guided_uninstall(hostname.as_deref())?;
            }
        }
        Provision {
            portainer_host,
            portainer_edition,
        } => {
            provision::handle_provision(hostname.as_deref(), portainer_host, &portainer_edition)?;
        }
        Smb { uninstall } => {
            smb::handle_smb(hostname.as_deref(), uninstall)?;
        }
        Docker { diagnose } => {
            if diagnose {
                docker::diagnose_docker(hostname.as_deref())?;
            } else {
                let target_host = hostname.as_deref().unwrap_or("localhost");
                docker::handle_docker(target_host)?;
            }
        }
        Npm {
            compose_file,
            service,
        } => {
            npm::handle_npm(hostname.as_deref(), &compose_file, service.as_deref())?;
        }
        Vpn { command } => {
            pia_vpn::handle_vpn(command)?;
        }
        Update {
            experimental,
            force,
        } => {
            update::handle_update(experimental, force)?;
        }
        Config {
            verbose,
            db,
            command,
        } => {
            config::handle_config(None, verbose, db, command.as_ref())?;
        }
        Db { command } => {
            config::handle_db_command(command)?;
        }
    }
    Ok(())
}

// Re-export command enums for convenience (these are used in main.rs)
// Note: These are re-exported from their respective modules, not defined here
