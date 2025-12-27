// Auto-generated module declarations
// This file is generated - do not edit manually
// Run `halvor db generate` to regenerate

pub mod agent_peers;
pub mod encrypted_env_data;
pub mod host_info;
pub mod join_tokens;
pub mod peer_keys;
pub mod settings;
pub mod smb_servers;
pub mod update_history;

// Re-export all generated structs
pub use agent_peers::{AgentPeersRow, AgentPeersRowData};
pub use encrypted_env_data::{EncryptedEnvDataRow, EncryptedEnvDataRowData};
pub use host_info::{HostInfoRow, HostInfoRowData};
pub use join_tokens::{JoinTokensRow, JoinTokensRowData};
pub use peer_keys::{PeerKeysRow, PeerKeysRowData};
pub use settings::{SettingsRow, SettingsRowData};
pub use smb_servers::{SmbServersRow, SmbServersRowData};
pub use update_history::{UpdateHistoryRow, UpdateHistoryRowData};

// Re-export wrapper functions with unique names
// Generic CRUD functions (insert_one, select_one, etc.) are accessible via module paths:
// e.g., db::settings::insert_one() or db::host_info::insert_one()

// Settings wrapper functions
pub use settings::{get_setting, set_setting};

// Host info wrapper functions
pub use host_info::{
    delete_host_config, get_host_config, get_host_info, list_hosts, store_host_config,
    store_host_info,
};

// SMB servers wrapper functions
pub use smb_servers::{delete_smb_server, get_smb_server, list_smb_servers, store_smb_server};

// Update history wrapper functions
pub use update_history::{get_update_history, record_update};

// Encrypted env data wrapper functions
pub use encrypted_env_data::{
    export_encrypted_data, get_all_encrypted_envs, get_encrypted_env, import_encrypted_data,
    store_encrypted_env,
};
