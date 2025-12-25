// Command module routing
//
// To add a new command:
// 1. Create a new file in this directory (e.g., `mycommand.rs`)
// 2. Add `pub mod mycommand;` below
// 3. Add the match arm in `handle_command` function

// Declare all command modules - add new modules here
pub mod agent;
pub mod backup;
pub mod build;
pub mod config;
pub mod configure;
pub mod dev;
pub mod generate;
pub mod init;
pub mod install;
pub mod join;
pub mod list;
pub mod status;
pub mod sync;
pub mod uninstall;
pub mod update;
pub mod utils;

use crate::Commands;
use crate::Commands::*;
use anyhow::Result;
use std::mem;

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
        Install { app, list, helm } => {
            install::handle_install(hostname.as_deref(), app.as_deref(), list, helm)?;
        }
        Uninstall { service } => {
            if let Some(service) = service {
                uninstall::handle_uninstall(hostname.as_deref(), &service)?;
            } else {
                uninstall::handle_guided_uninstall(hostname.as_deref())?;
            }
        }
        Update {
            app,
            experimental,
            force,
        } => {
            update::handle_update(hostname.as_deref(), app.as_deref(), experimental, force)?;
        }
        Init { token, yes } => {
            let target_host = hostname.as_deref().unwrap_or("localhost");
            init::handle_init(target_host, token.as_deref(), yes)?;
        }
        Config {
            verbose,
            db,
            command,
        } => {
            // Convert Option<halvor::commands::config::ConfigCommands> to Option<commands::config::ConfigCommands>
            let local_command =
                command.map(|c| unsafe { mem::transmute::<_, config::ConfigCommands>(c) });
            config::handle_config(None, verbose, db, local_command.as_ref())?;
        }
        Db { command } => {
            let local_command: config::DbCommands = unsafe { mem::transmute(command) };
            config::handle_db_command(local_command)?;
        }
        Build { command } => {
            let local_command: build::BuildCommands = unsafe { mem::transmute(command) };
            build::handle_build(local_command)?;
        }
        Dev { command } => {
            let rt = tokio::runtime::Runtime::new()?;
            let local_command: dev::DevCommands = unsafe { mem::transmute(command) };
            rt.block_on(dev::handle_dev(local_command))?;
        }
        Generate { command } => {
            let local_command: generate::GenerateCommands = unsafe { mem::transmute(command) };
            generate::handle_generate(local_command)?;
        }
        Join {
            join_hostname,
            server,
            token,
            control_plane,
        } => {
            join::handle_join(
                hostname.as_deref(),
                join_hostname,
                server,
                token,
                control_plane,
            )?;
        }
        Status { command } => {
            let local_command: status::StatusCommands = unsafe { mem::transmute(command) };
            status::handle_status(hostname.as_deref(), local_command)?;
        }
        Configure { hostname: target_host } => {
            configure::handle_configure(hostname.as_deref().or(target_host.as_deref()))?;
        }
    }
    Ok(())
}

// Re-export command enums for convenience (these are used in main.rs)
// Note: These are re-exported from their respective modules, not defined here
