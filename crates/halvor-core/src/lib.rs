// Halvor Core Library
// Shared business logic and utilities

// Re-export halvor-db
pub use halvor_db as db;

// Core modules
pub mod apps;
pub mod config;
pub mod services;
pub mod utils;

// Re-export commonly used items
pub use config::ConfigManager;

// Re-export db helpers for convenience (from halvor-db)
pub use halvor_db::{
    get_host_config, store_host_config, delete_host_config,
    get_smb_server, store_smb_server, delete_smb_server,
    get_encrypted_env, store_encrypted_env, get_all_encrypted_envs,
};
