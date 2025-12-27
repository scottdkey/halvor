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
        Install {
            app,
            list,
            repo,
            repo_name,
        } => {
            install::handle_install(
                hostname.as_deref(),
                app.as_deref(),
                list,
                repo.as_deref(),
                repo_name.as_deref(),
            )?;
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
        Init {
            token,
            yes,
            skip_k3s,
        } => {
            // Detect current machine's hostname if not provided
            let halvor_dir = crate::config::find_halvor_dir()?;
            let config = crate::config::load_env_config(&halvor_dir)?;

            let target_host = if let Some(host) = hostname.as_deref() {
                host.to_string()
            } else {
                // Try to detect current hostname and find it in config
                match crate::config::service::get_current_hostname() {
                    Ok(current_host) => {
                        // Try to find it in config (with normalization)
                        if let Some(found_host) =
                            crate::config::service::find_hostname_in_config(&current_host, &config)
                        {
                            found_host
                        } else {
                            // Not in config, but we can still use it - Executor will detect it's local
                            current_host
                        }
                    }
                    Err(_) => {
                        // Fallback to localhost if we can't detect hostname
                        "localhost".to_string()
                    }
                }
            };
            init::handle_init(&target_host, token.as_deref(), yes, skip_k3s)?;
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
            let local_command: Option<status::StatusCommands> =
                command.map(|c| unsafe { mem::transmute(c) });
            status::handle_status(hostname.as_deref(), local_command)?;
        }
        Agent { command } => {
            let rt = tokio::runtime::Runtime::new()?;
            let local_command: agent::AgentCommands = unsafe { mem::transmute(command) };
            rt.block_on(agent::handle_agent(local_command))?;
        }
    }
    Ok(())
}

// Re-export command enums for convenience (these are used in main.rs)
// Note: These are re-exported from their respective modules, not defined here
