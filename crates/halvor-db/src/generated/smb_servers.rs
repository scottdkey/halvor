// Auto-generated from database schema
// This file is generated - do not edit manually
// Run `halvor db generate` to regenerate

use crate::core::table::DbTable;
use crate::impl_table_auto;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct SmbServersRow {
    pub id: String,
    pub server_name: Option<String>,
    pub host: String,
    pub shares: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub options: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

// Automatically implement Table trait from struct definition
impl_table_auto!(
    SmbServersRow,
    "smb_servers",
    [server_name, host, shares, username, password, options]
);

/// Data structure for SmbServersRow operations (excludes id, created_at, updated_at)
#[derive(Debug, Clone)]
pub struct SmbServersRowData {
    pub server_name: Option<String>,
    pub host: String,
    pub shares: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub options: Option<String>,
}

/// Insert a new SmbServersRow record
/// Only data fields are required - id, created_at, and updated_at are set automatically
pub fn insert_one(data: SmbServersRowData) -> Result<String> {
    let conn = crate::get_connection()?;
    let row = SmbServersRow {
        id: String::new(), // Set automatically
        server_name: data.server_name.clone(),
        host: data.host.clone(),
        shares: data.shares.clone(),
        username: data.username.clone(),
        password: data.password.clone(),
        options: data.options.clone(),

        created_at: 0, // Set automatically
        updated_at: 0, // Set automatically
    };
    DbTable::<SmbServersRow>::insert(&conn, &row)
}

/// Insert multiple SmbServersRow records
pub fn insert_many(data_vec: Vec<SmbServersRowData>) -> Result<Vec<String>> {
    let conn = crate::get_connection()?;
    let mut ids = Vec::new();
    for data in data_vec {
        let row = SmbServersRow {
            id: String::new(), // Set automatically
            server_name: data.server_name.clone(),
            host: data.host.clone(),
            shares: data.shares.clone(),
            username: data.username.clone(),
            password: data.password.clone(),
            options: data.options.clone(),

            created_at: 0, // Set automatically
            updated_at: 0, // Set automatically
        };
        ids.push(DbTable::<SmbServersRow>::insert(&conn, &row)?);
    }
    Ok(ids)
}

/// Upsert a SmbServersRow record (insert if new, update if exists)
/// Only data fields are required - id, created_at, and updated_at are handled automatically
pub fn upsert_one(
    where_clause: &str,
    where_params: &[&dyn rusqlite::types::ToSql],
    data: SmbServersRowData,
) -> Result<String> {
    let conn = crate::get_connection()?;
    DbTable::<SmbServersRow>::upsert_by(&conn, where_clause, where_params, |existing| {
        let mut row = existing.cloned().unwrap_or_else(|| {
            let mut r = SmbServersRow {
                id: String::new(), // Set automatically
                server_name: None,
                host: String::new(),
                shares: String::new(),
                username: None,
                password: None,
                options: None,

                created_at: 0, // Set automatically
                updated_at: 0, // Set automatically
            };
            // Set initial values from data
            r.server_name = data.server_name.clone();
            r.host = data.host.clone();
            r.shares = data.shares.clone();
            r.username = data.username.clone();
            r.password = data.password.clone();
            r.options = data.options.clone();

            r
        });
        // Update only the data fields
        row.server_name = data.server_name;
        row.host = data.host;
        row.shares = data.shares;
        row.username = data.username;
        row.password = data.password;
        row.options = data.options;

        row
    })
}

/// Select one SmbServersRow record
pub fn select_one(
    where_clause: &str,
    params: &[&dyn rusqlite::types::ToSql],
) -> Result<Option<SmbServersRow>> {
    let conn = crate::get_connection()?;
    DbTable::<SmbServersRow>::select_one(&conn, where_clause, params)
}

/// Select many SmbServersRow records
pub fn select_many(
    where_clause: &str,
    params: &[&dyn rusqlite::types::ToSql],
) -> Result<Vec<SmbServersRow>> {
    let conn = crate::get_connection()?;
    DbTable::<SmbServersRow>::select_many(&conn, where_clause, params)
}

/// Delete SmbServersRow record by primary key (id)
pub fn delete_by_id(id: &str) -> Result<usize> {
    let conn = crate::get_connection()?;
    DbTable::<SmbServersRow>::delete_many(&conn, "id = ?1", &[&id as &dyn rusqlite::types::ToSql])
}

/// Delete SmbServersRow record by unique key: server_name
pub fn delete_by_server_name(server_name_value: &str) -> Result<usize> {
    let conn = crate::get_connection()?;
    DbTable::<SmbServersRow>::delete_many(
        &conn,
        "server_name = ?1",
        &[&server_name_value as &dyn rusqlite::types::ToSql],
    )
}

// NOTE: Config-dependent functions moved to halvor core crate
// These functions depend on config::SmbServerConfig which is in halvor-core
// They are now in halvor-cli/src/config/service.rs

use serde_json;

/// List all SMB server names in database
pub fn list_smb_servers() -> Result<Vec<String>> {
    let rows = select_many("1=1", &[])?;
    Ok(rows.into_iter().filter_map(|r| r.server_name).collect())
}
