pub mod core;
pub mod generated;
pub mod migrate;
pub mod migrations;

use crate::config::config_manager;
use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::PathBuf;

const DB_FILE_NAME: &str = "halvor.db";

/// Get the database file path (in the config directory)
pub fn get_db_path() -> Result<PathBuf> {
    let config_dir = config_manager::get_config_dir()?;
    Ok(config_dir.join(DB_FILE_NAME))
}

/// Initialize the database and run migrations
///
/// This function automatically runs all pending migrations when the database is first accessed.
/// Migrations are run sequentially in order, ensuring the database schema is always up to date.
pub fn init_db() -> Result<Connection> {
    let db_path = get_db_path()?;
    let conn = Connection::open(&db_path)
        .with_context(|| format!("Failed to open database: {}", db_path.display()))?;

    // Run migrations to set up/update schema
    // This happens automatically on every database access to ensure schema is current
    migrations::run_migrations(&conn)?;

    Ok(conn)
}

/// Get a database connection
pub fn get_connection() -> Result<Connection> {
    init_db()
}

/// Get a database client for custom SQL queries
pub fn get_client() -> Result<core::DbClient> {
    let conn = get_connection()?;
    Ok(core::DbClient::new(conn))
}

// Re-export generated modules directly for convenience
// This allows calling db::settings::insert_one() instead of db::generated::settings::insert_one()
pub mod settings {
    pub use super::generated::settings::*;
}

pub mod host_info {
    pub use super::generated::host_info::*;
}

pub mod smb_servers {
    pub use super::generated::smb_servers::*;
}

pub mod update_history {
    pub use super::generated::update_history::*;
}

pub mod encrypted_env_data {
    pub use super::generated::encrypted_env_data::*;
}

// Re-export wrapper functions with unique names at the top level for convenience
// These can be called directly via db::get_host_config(), etc.
// Note: Generic CRUD functions are accessible via module paths like db::settings::insert_one()
pub use generated::{
    delete_host_config, get_host_config, get_host_info, get_setting, list_hosts, set_setting,
    store_host_config, store_host_info,
};
pub use generated::{delete_smb_server, get_smb_server, list_smb_servers, store_smb_server};
pub use generated::{
    export_encrypted_data, get_all_encrypted_envs, get_encrypted_env, import_encrypted_data,
    store_encrypted_env,
};
pub use generated::{get_update_history, record_update};
