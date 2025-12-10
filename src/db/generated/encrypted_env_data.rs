// Auto-generated from database schema
// This file is generated - do not edit manually
// Run `halvor db generate` to regenerate

use crate::db;
use crate::db::core::table::DbTable;
use crate::impl_table_auto;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedEnvDataRow {
    pub id: String,
    pub hostname: Option<String>,
    pub key: String,
    pub encrypted_value: String,
    pub created_at: i64,
    pub updated_at: i64,
}

// Automatically implement Table trait from struct definition
impl_table_auto!(
    EncryptedEnvDataRow,
    "encrypted_env_data",
    [hostname, key, encrypted_value]
);

/// Data structure for EncryptedEnvDataRow operations (excludes id, created_at, updated_at)
#[derive(Debug, Clone)]
pub struct EncryptedEnvDataRowData {
    pub hostname: Option<String>,
    pub key: String,
    pub encrypted_value: String,
}

/// Insert a new EncryptedEnvDataRow record
/// Only data fields are required - id, created_at, and updated_at are set automatically
pub fn insert_one(data: EncryptedEnvDataRowData) -> Result<String> {
    let conn = db::get_connection()?;
    let row = EncryptedEnvDataRow {
        id: String::new(), // Set automatically
        hostname: data.hostname.clone(),
        key: data.key.clone(),
        encrypted_value: data.encrypted_value.clone(),

        created_at: 0, // Set automatically
        updated_at: 0, // Set automatically
    };
    DbTable::<EncryptedEnvDataRow>::insert(&conn, &row)
}

/// Insert multiple EncryptedEnvDataRow records
pub fn insert_many(data_vec: Vec<EncryptedEnvDataRowData>) -> Result<Vec<String>> {
    let conn = db::get_connection()?;
    let mut ids = Vec::new();
    for data in data_vec {
        let row = EncryptedEnvDataRow {
            id: String::new(), // Set automatically
            hostname: data.hostname.clone(),
            key: data.key.clone(),
            encrypted_value: data.encrypted_value.clone(),

            created_at: 0, // Set automatically
            updated_at: 0, // Set automatically
        };
        ids.push(DbTable::<EncryptedEnvDataRow>::insert(&conn, &row)?);
    }
    Ok(ids)
}

/// Upsert a EncryptedEnvDataRow record (insert if new, update if exists)
/// Only data fields are required - id, created_at, and updated_at are handled automatically
pub fn upsert_one(
    where_clause: &str,
    where_params: &[&dyn rusqlite::types::ToSql],
    data: EncryptedEnvDataRowData,
) -> Result<String> {
    let conn = db::get_connection()?;
    DbTable::<EncryptedEnvDataRow>::upsert_by(&conn, where_clause, where_params, |existing| {
        let mut row = existing.cloned().unwrap_or_else(|| {
            let mut r = EncryptedEnvDataRow {
                id: String::new(), // Set automatically
                hostname: None,
                key: String::new(),
                encrypted_value: String::new(),

                created_at: 0, // Set automatically
                updated_at: 0, // Set automatically
            };
            // Set initial values from data
            r.hostname = data.hostname.clone();
            r.key = data.key.clone();
            r.encrypted_value = data.encrypted_value.clone();

            r
        });
        // Update only the data fields
        row.hostname = data.hostname;
        row.key = data.key;
        row.encrypted_value = data.encrypted_value;

        row
    })
}

/// Select one EncryptedEnvDataRow record
pub fn select_one(
    where_clause: &str,
    params: &[&dyn rusqlite::types::ToSql],
) -> Result<Option<EncryptedEnvDataRow>> {
    let conn = db::get_connection()?;
    DbTable::<EncryptedEnvDataRow>::select_one(&conn, where_clause, params)
}

/// Select many EncryptedEnvDataRow records
pub fn select_many(
    where_clause: &str,
    params: &[&dyn rusqlite::types::ToSql],
) -> Result<Vec<EncryptedEnvDataRow>> {
    let conn = db::get_connection()?;
    DbTable::<EncryptedEnvDataRow>::select_many(&conn, where_clause, params)
}

/// Delete EncryptedEnvDataRow record by primary key (id)
pub fn delete_by_id(id: &str) -> Result<usize> {
    let conn = db::get_connection()?;
    DbTable::<EncryptedEnvDataRow>::delete_many(
        &conn,
        "id = ?1",
        &[&id as &dyn rusqlite::types::ToSql],
    )
}

use crate::utils::crypto;
use anyhow::Context;

/// Store encrypted environment variable
pub fn store_encrypted_env(hostname: Option<&str>, key: &str, value: &str) -> Result<()> {
    let encrypted = crypto::encrypt(value)?;
    upsert_one(
        "hostname IS ?1 AND key = ?2",
        &[
            &hostname as &dyn rusqlite::types::ToSql,
            &key as &dyn rusqlite::types::ToSql,
        ],
        EncryptedEnvDataRowData {
            hostname: hostname.map(|s| s.to_string()),
            key: key.to_string(),
            encrypted_value: encrypted,
        },
    )?;
    Ok(())
}

/// Get encrypted environment variable
pub fn get_encrypted_env(hostname: Option<&str>, key: &str) -> Result<Option<String>> {
    let row = select_one(
        "hostname IS ?1 AND key = ?2",
        &[
            &hostname as &dyn rusqlite::types::ToSql,
            &key as &dyn rusqlite::types::ToSql,
        ],
    )?;
    Ok(row.and_then(|r| crypto::decrypt(&r.encrypted_value).ok()))
}

/// Get all encrypted environment variables for a hostname (or global if None)
pub fn get_all_encrypted_envs(hostname: Option<&str>) -> Result<Vec<(String, String)>> {
    let rows = select_many(
        "hostname IS ?1",
        &[&hostname as &dyn rusqlite::types::ToSql],
    )?;
    let mut envs = Vec::new();
    for row in rows {
        if let Ok(decrypted) = crypto::decrypt(&row.encrypted_value) {
            envs.push((row.key, decrypted));
        }
    }
    Ok(envs)
}

/// Export all encrypted data for sync
pub fn export_encrypted_data() -> Result<Vec<u8>> {
    let data = select_many("1=1", &[])?;
    let json = serde_json::to_string(&data).context("Failed to serialize encrypted data")?;
    Ok(json.into_bytes())
}

/// Import encrypted data from sync
pub fn import_encrypted_data(data: &[u8]) -> Result<()> {
    let rows: Vec<EncryptedEnvDataRow> =
        serde_json::from_slice(data).context("Failed to parse encrypted data")?;
    for row in rows {
        upsert_one(
            "hostname IS ?1 AND key = ?2",
            &[
                &row.hostname.as_deref() as &dyn rusqlite::types::ToSql,
                &row.key as &dyn rusqlite::types::ToSql,
            ],
            EncryptedEnvDataRowData {
                hostname: row.hostname.clone(),
                key: row.key.clone(),
                encrypted_value: row.encrypted_value.clone(),
            },
        )?;
    }
    Ok(())
}
