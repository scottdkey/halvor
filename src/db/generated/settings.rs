// Auto-generated from database schema
// This file is generated - do not edit manually
// Run `halvor db generate` to regenerate

use crate::db;
use crate::db::core::table::DbTable;
use crate::impl_table_auto;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct SettingsRow {
    pub id: String,
    pub key: Option<String>,
    pub value: String,
    pub created_at: i64,
    pub updated_at: i64,
}

// Automatically implement Table trait from struct definition
impl_table_auto!(SettingsRow, "settings", [key, value]);

/// Data structure for SettingsRow operations (excludes id, created_at, updated_at)
#[derive(Debug, Clone)]
pub struct SettingsRowData {
    pub key: Option<String>,
    pub value: String,
}

/// Insert a new SettingsRow record
/// Only data fields are required - id, created_at, and updated_at are set automatically
pub fn insert_one(data: SettingsRowData) -> Result<String> {
    let conn = db::get_connection()?;
    let row = SettingsRow {
        id: String::new(), // Set automatically
        key: data.key.clone(),
        value: data.value.clone(),

        created_at: 0, // Set automatically
        updated_at: 0, // Set automatically
    };
    DbTable::<SettingsRow>::insert(&conn, &row)
}

/// Insert multiple SettingsRow records
pub fn insert_many(data_vec: Vec<SettingsRowData>) -> Result<Vec<String>> {
    let conn = db::get_connection()?;
    let mut ids = Vec::new();
    for data in data_vec {
        let row = SettingsRow {
            id: String::new(), // Set automatically
            key: data.key.clone(),
            value: data.value.clone(),

            created_at: 0, // Set automatically
            updated_at: 0, // Set automatically
        };
        ids.push(DbTable::<SettingsRow>::insert(&conn, &row)?);
    }
    Ok(ids)
}

/// Upsert a SettingsRow record (insert if new, update if exists)
/// Only data fields are required - id, created_at, and updated_at are handled automatically
pub fn upsert_one(
    where_clause: &str,
    where_params: &[&dyn rusqlite::types::ToSql],
    data: SettingsRowData,
) -> Result<String> {
    let conn = db::get_connection()?;
    DbTable::<SettingsRow>::upsert_by(&conn, where_clause, where_params, |existing| {
        let mut row = existing.cloned().unwrap_or_else(|| {
            let mut r = SettingsRow {
                id: String::new(), // Set automatically
                key: None,
                value: String::new(),

                created_at: 0, // Set automatically
                updated_at: 0, // Set automatically
            };
            // Set initial values from data
            r.key = data.key.clone();
            r.value = data.value.clone();

            r
        });
        // Update only the data fields
        row.key = data.key;
        row.value = data.value;

        row
    })
}

/// Select one SettingsRow record
pub fn select_one(
    where_clause: &str,
    params: &[&dyn rusqlite::types::ToSql],
) -> Result<Option<SettingsRow>> {
    let conn = db::get_connection()?;
    DbTable::<SettingsRow>::select_one(&conn, where_clause, params)
}

/// Select many SettingsRow records
pub fn select_many(
    where_clause: &str,
    params: &[&dyn rusqlite::types::ToSql],
) -> Result<Vec<SettingsRow>> {
    let conn = db::get_connection()?;
    DbTable::<SettingsRow>::select_many(&conn, where_clause, params)
}

/// Delete SettingsRow record by primary key (id)
pub fn delete_by_id(id: &str) -> Result<usize> {
    let conn = db::get_connection()?;
    DbTable::<SettingsRow>::delete_many(&conn, "id = ?1", &[&id as &dyn rusqlite::types::ToSql])
}

/// Delete SettingsRow record by unique key: key
pub fn delete_by_key(key_value: &str) -> Result<usize> {
    let conn = db::get_connection()?;
    DbTable::<SettingsRow>::delete_many(
        &conn,
        "key = ?1",
        &[&key_value as &dyn rusqlite::types::ToSql],
    )
}

/// Set a setting value (convenience wrapper)
pub fn set_setting(key: &str, value: &str) -> Result<()> {
    upsert_one(
        "key = ?1",
        &[&key as &dyn rusqlite::types::ToSql],
        SettingsRowData {
            key: Some(key.to_string()),
            value: value.to_string(),
        },
    )?;
    Ok(())
}

/// Get a setting value (convenience wrapper)
pub fn get_setting(key: &str) -> Result<Option<String>> {
    let row = select_one("key = ?1", &[&key as &dyn rusqlite::types::ToSql])?;
    Ok(row.map(|r| r.value))
}
