// Auto-generated from database schema
// This file is generated - do not edit manually
// Run `halvor db generate` to regenerate

use crate::impl_table_auto;
use crate::db;
use crate::db::core::table::DbTable;
use anyhow::Result;


#[derive(Debug, Clone)]
pub struct AgentPeersRow {
    pub id: String,
    pub hostname: String,
    pub tailscale_ip: Option<String>,
    pub tailscale_hostname: Option<String>,
    pub public_key: String,
    pub status: String,
    pub last_seen_at: Option<i64>,
    pub joined_at: i64,
    pub created_at: i64,
    pub updated_at: i64,

}

// Automatically implement Table trait from struct definition
impl_table_auto!(
    AgentPeersRow,
    "agent_peers",
    [hostname, tailscale_ip, tailscale_hostname, public_key, status, last_seen_at, joined_at]
);


/// Data structure for AgentPeersRow operations (excludes id, created_at, updated_at)
#[derive(Debug, Clone)]
pub struct AgentPeersRowData {
    pub hostname: String,
    pub tailscale_ip: Option<String>,
    pub tailscale_hostname: Option<String>,
    pub public_key: String,
    pub status: String,
    pub last_seen_at: Option<i64>,
    pub joined_at: i64,

}

/// Insert a new AgentPeersRow record
/// Only data fields are required - id, created_at, and updated_at are set automatically
pub fn insert_one(data: AgentPeersRowData) -> Result<String> {
    let conn = db::get_connection()?;
    let row = AgentPeersRow {
        id: String::new(), // Set automatically
        hostname: data.hostname.clone(),
        tailscale_ip: data.tailscale_ip.clone(),
        tailscale_hostname: data.tailscale_hostname.clone(),
        public_key: data.public_key.clone(),
        status: data.status.clone(),
        last_seen_at: data.last_seen_at.clone(),
        joined_at: data.joined_at.clone(),

        created_at: 0, // Set automatically
        updated_at: 0, // Set automatically
    };
    DbTable::<AgentPeersRow>::insert(&conn, &row)
}

/// Insert multiple AgentPeersRow records
pub fn insert_many(data_vec: Vec<AgentPeersRowData>) -> Result<Vec<String>> {
    let conn = db::get_connection()?;
    let mut ids = Vec::new();
    for data in data_vec {
        let row = AgentPeersRow {
            id: String::new(), // Set automatically
        hostname: data.hostname.clone(),
        tailscale_ip: data.tailscale_ip.clone(),
        tailscale_hostname: data.tailscale_hostname.clone(),
        public_key: data.public_key.clone(),
        status: data.status.clone(),
        last_seen_at: data.last_seen_at.clone(),
        joined_at: data.joined_at.clone(),

            created_at: 0, // Set automatically
            updated_at: 0, // Set automatically
        };
        ids.push(DbTable::<AgentPeersRow>::insert(&conn, &row)?);
    }
    Ok(ids)
}

/// Upsert a AgentPeersRow record (insert if new, update if exists)
/// Only data fields are required - id, created_at, and updated_at are handled automatically
pub fn upsert_one(where_clause: &str, where_params: &[&dyn rusqlite::types::ToSql], data: AgentPeersRowData) -> Result<String> {
    let conn = db::get_connection()?;
    DbTable::<AgentPeersRow>::upsert_by(
        &conn,
        where_clause,
        where_params,
        |existing| {
            let mut row = existing.cloned().unwrap_or_else(|| {
                let mut r = AgentPeersRow {
                    id: String::new(), // Set automatically
                hostname: String::new(),
                tailscale_ip: None,
                tailscale_hostname: None,
                public_key: String::new(),
                status: String::new(),
                last_seen_at: None,
                joined_at: 0,

                    created_at: 0, // Set automatically
                    updated_at: 0, // Set automatically
                };
                // Set initial values from data
                r.hostname = data.hostname.clone();
                r.tailscale_ip = data.tailscale_ip.clone();
                r.tailscale_hostname = data.tailscale_hostname.clone();
                r.public_key = data.public_key.clone();
                r.status = data.status.clone();
                r.last_seen_at = data.last_seen_at.clone();
                r.joined_at = data.joined_at.clone();

                r
            });
            // Update only the data fields
            row.hostname = data.hostname;
            row.tailscale_ip = data.tailscale_ip;
            row.tailscale_hostname = data.tailscale_hostname;
            row.public_key = data.public_key;
            row.status = data.status;
            row.last_seen_at = data.last_seen_at;
            row.joined_at = data.joined_at;

            row
        },
    )
}

/// Select one AgentPeersRow record
pub fn select_one(where_clause: &str, params: &[&dyn rusqlite::types::ToSql]) -> Result<Option<AgentPeersRow>> {
    let conn = db::get_connection()?;
    DbTable::<AgentPeersRow>::select_one(&conn, where_clause, params)
}

/// Select many AgentPeersRow records
pub fn select_many(where_clause: &str, params: &[&dyn rusqlite::types::ToSql]) -> Result<Vec<AgentPeersRow>> {
    let conn = db::get_connection()?;
    DbTable::<AgentPeersRow>::select_many(&conn, where_clause, params)
}

/// Delete AgentPeersRow record by primary key (id)
pub fn delete_by_id(id: &str) -> Result<usize> {
    let conn = db::get_connection()?;
    DbTable::<AgentPeersRow>::delete_many(&conn, "id = ?1", &[&id as &dyn rusqlite::types::ToSql])
}

/// Delete AgentPeersRow record by unique key: hostname
pub fn delete_by_hostname(hostname_value: &str) -> Result<usize> {
    let conn = db::get_connection()?;
    DbTable::<AgentPeersRow>::delete_many(&conn, "hostname = ?1", &[&hostname_value as &dyn rusqlite::types::ToSql])
}


/// Delete AgentPeersRow record by unique key: id
pub fn delete_by_id(id_value: &str) -> Result<usize> {
    let conn = db::get_connection()?;
    DbTable::<AgentPeersRow>::delete_many(&conn, "id = ?1", &[&id_value as &dyn rusqlite::types::ToSql])
}


