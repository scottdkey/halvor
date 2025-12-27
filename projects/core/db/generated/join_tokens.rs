// Auto-generated from database schema
// This file is generated - do not edit manually
// Run `halvor db generate` to regenerate

use crate::impl_table_auto;
use crate::db;
use crate::db::core::table::DbTable;
use anyhow::Result;


#[derive(Debug, Clone)]
pub struct JoinTokensRow {
    pub id: String,
    pub token: String,
    pub issuer_hostname: String,
    pub expires_at: i64,
    pub used: i64,
    pub used_by_hostname: Option<String>,
    pub used_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,

}

// Automatically implement Table trait from struct definition
impl_table_auto!(
    JoinTokensRow,
    "join_tokens",
    [token, issuer_hostname, expires_at, used, used_by_hostname, used_at]
);


/// Data structure for JoinTokensRow operations (excludes id, created_at, updated_at)
#[derive(Debug, Clone)]
pub struct JoinTokensRowData {
    pub token: String,
    pub issuer_hostname: String,
    pub expires_at: i64,
    pub used: i64,
    pub used_by_hostname: Option<String>,
    pub used_at: Option<i64>,

}

/// Insert a new JoinTokensRow record
/// Only data fields are required - id, created_at, and updated_at are set automatically
pub fn insert_one(data: JoinTokensRowData) -> Result<String> {
    let conn = db::get_connection()?;
    let row = JoinTokensRow {
        id: String::new(), // Set automatically
        token: data.token.clone(),
        issuer_hostname: data.issuer_hostname.clone(),
        expires_at: data.expires_at.clone(),
        used: data.used.clone(),
        used_by_hostname: data.used_by_hostname.clone(),
        used_at: data.used_at.clone(),

        created_at: 0, // Set automatically
        updated_at: 0, // Set automatically
    };
    DbTable::<JoinTokensRow>::insert(&conn, &row)
}

/// Insert multiple JoinTokensRow records
pub fn insert_many(data_vec: Vec<JoinTokensRowData>) -> Result<Vec<String>> {
    let conn = db::get_connection()?;
    let mut ids = Vec::new();
    for data in data_vec {
        let row = JoinTokensRow {
            id: String::new(), // Set automatically
        token: data.token.clone(),
        issuer_hostname: data.issuer_hostname.clone(),
        expires_at: data.expires_at.clone(),
        used: data.used.clone(),
        used_by_hostname: data.used_by_hostname.clone(),
        used_at: data.used_at.clone(),

            created_at: 0, // Set automatically
            updated_at: 0, // Set automatically
        };
        ids.push(DbTable::<JoinTokensRow>::insert(&conn, &row)?);
    }
    Ok(ids)
}

/// Upsert a JoinTokensRow record (insert if new, update if exists)
/// Only data fields are required - id, created_at, and updated_at are handled automatically
pub fn upsert_one(where_clause: &str, where_params: &[&dyn rusqlite::types::ToSql], data: JoinTokensRowData) -> Result<String> {
    let conn = db::get_connection()?;
    DbTable::<JoinTokensRow>::upsert_by(
        &conn,
        where_clause,
        where_params,
        |existing| {
            let mut row = existing.cloned().unwrap_or_else(|| {
                let mut r = JoinTokensRow {
                    id: String::new(), // Set automatically
                token: String::new(),
                issuer_hostname: String::new(),
                expires_at: 0,
                used: 0,
                used_by_hostname: None,
                used_at: None,

                    created_at: 0, // Set automatically
                    updated_at: 0, // Set automatically
                };
                // Set initial values from data
                r.token = data.token.clone();
                r.issuer_hostname = data.issuer_hostname.clone();
                r.expires_at = data.expires_at.clone();
                r.used = data.used.clone();
                r.used_by_hostname = data.used_by_hostname.clone();
                r.used_at = data.used_at.clone();

                r
            });
            // Update only the data fields
            row.token = data.token;
            row.issuer_hostname = data.issuer_hostname;
            row.expires_at = data.expires_at;
            row.used = data.used;
            row.used_by_hostname = data.used_by_hostname;
            row.used_at = data.used_at;

            row
        },
    )
}

/// Select one JoinTokensRow record
pub fn select_one(where_clause: &str, params: &[&dyn rusqlite::types::ToSql]) -> Result<Option<JoinTokensRow>> {
    let conn = db::get_connection()?;
    DbTable::<JoinTokensRow>::select_one(&conn, where_clause, params)
}

/// Select many JoinTokensRow records
pub fn select_many(where_clause: &str, params: &[&dyn rusqlite::types::ToSql]) -> Result<Vec<JoinTokensRow>> {
    let conn = db::get_connection()?;
    DbTable::<JoinTokensRow>::select_many(&conn, where_clause, params)
}

/// Delete JoinTokensRow record by primary key (id)
pub fn delete_by_id(id: &str) -> Result<usize> {
    let conn = db::get_connection()?;
    DbTable::<JoinTokensRow>::delete_many(&conn, "id = ?1", &[&id as &dyn rusqlite::types::ToSql])
}

/// Delete JoinTokensRow record by unique key: token
pub fn delete_by_token(token_value: &str) -> Result<usize> {
    let conn = db::get_connection()?;
    DbTable::<JoinTokensRow>::delete_many(&conn, "token = ?1", &[&token_value as &dyn rusqlite::types::ToSql])
}


