// Auto-generated from database schema
// This file is generated - do not edit manually
// Run `halvor db generate` to regenerate

use crate::impl_table_auto;
use crate::core::table::DbTable;
use anyhow::Result;


#[derive(Debug, Clone)]
pub struct PeerKeysRow {
    pub id: String,
    pub peer_hostname: String,
    pub shared_secret: String,
    pub algorithm: String,
    pub created_at: i64,
    pub updated_at: i64,

}

// Automatically implement Table trait from struct definition
impl_table_auto!(
    PeerKeysRow,
    "peer_keys",
    [peer_hostname, shared_secret, algorithm]
);


/// Data structure for PeerKeysRow operations (excludes id, created_at, updated_at)
#[derive(Debug, Clone)]
pub struct PeerKeysRowData {
    pub peer_hostname: String,
    pub shared_secret: String,
    pub algorithm: String,

}

/// Insert a new PeerKeysRow record
/// Only data fields are required - id, created_at, and updated_at are set automatically
pub fn insert_one(data: PeerKeysRowData) -> Result<String> {
    let conn = crate::get_connection()?;
    let row = PeerKeysRow {
        id: String::new(), // Set automatically
        peer_hostname: data.peer_hostname.clone(),
        shared_secret: data.shared_secret.clone(),
        algorithm: data.algorithm.clone(),

        created_at: 0, // Set automatically
        updated_at: 0, // Set automatically
    };
    DbTable::<PeerKeysRow>::insert(&conn, &row)
}

/// Insert multiple PeerKeysRow records
pub fn insert_many(data_vec: Vec<PeerKeysRowData>) -> Result<Vec<String>> {
    let conn = crate::get_connection()?;
    let mut ids = Vec::new();
    for data in data_vec {
        let row = PeerKeysRow {
            id: String::new(), // Set automatically
        peer_hostname: data.peer_hostname.clone(),
        shared_secret: data.shared_secret.clone(),
        algorithm: data.algorithm.clone(),

            created_at: 0, // Set automatically
            updated_at: 0, // Set automatically
        };
        ids.push(DbTable::<PeerKeysRow>::insert(&conn, &row)?);
    }
    Ok(ids)
}

/// Upsert a PeerKeysRow record (insert if new, update if exists)
/// Only data fields are required - id, created_at, and updated_at are handled automatically
pub fn upsert_one(where_clause: &str, where_params: &[&dyn rusqlite::types::ToSql], data: PeerKeysRowData) -> Result<String> {
    let conn = crate::get_connection()?;
    DbTable::<PeerKeysRow>::upsert_by(
        &conn,
        where_clause,
        where_params,
        |existing| {
            let mut row = existing.cloned().unwrap_or_else(|| {
                let mut r = PeerKeysRow {
                    id: String::new(), // Set automatically
                peer_hostname: String::new(),
                shared_secret: String::new(),
                algorithm: String::new(),

                    created_at: 0, // Set automatically
                    updated_at: 0, // Set automatically
                };
                // Set initial values from data
                r.peer_hostname = data.peer_hostname.clone();
                r.shared_secret = data.shared_secret.clone();
                r.algorithm = data.algorithm.clone();

                r
            });
            // Update only the data fields
            row.peer_hostname = data.peer_hostname;
            row.shared_secret = data.shared_secret;
            row.algorithm = data.algorithm;

            row
        },
    )
}

/// Select one PeerKeysRow record
pub fn select_one(where_clause: &str, params: &[&dyn rusqlite::types::ToSql]) -> Result<Option<PeerKeysRow>> {
    let conn = crate::get_connection()?;
    DbTable::<PeerKeysRow>::select_one(&conn, where_clause, params)
}

/// Select many PeerKeysRow records
pub fn select_many(where_clause: &str, params: &[&dyn rusqlite::types::ToSql]) -> Result<Vec<PeerKeysRow>> {
    let conn = crate::get_connection()?;
    DbTable::<PeerKeysRow>::select_many(&conn, where_clause, params)
}

/// Delete PeerKeysRow record by primary key (id)
pub fn delete_by_id(id: &str) -> Result<usize> {
    let conn = crate::get_connection()?;
    DbTable::<PeerKeysRow>::delete_many(&conn, "id = ?1", &[&id as &dyn rusqlite::types::ToSql])
}

/// Delete PeerKeysRow record by unique key: peer_hostname
pub fn delete_by_peer_hostname(peer_hostname_value: &str) -> Result<usize> {
    let conn = crate::get_connection()?;
    DbTable::<PeerKeysRow>::delete_many(&conn, "peer_hostname = ?1", &[&peer_hostname_value as &dyn rusqlite::types::ToSql])
}


