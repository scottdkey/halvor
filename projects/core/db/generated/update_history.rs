// Auto-generated from database schema
// This file is generated - do not edit manually
// Run `halvor db generate` to regenerate

use crate::db;
use crate::db::core::table::DbTable;
use crate::impl_table_auto;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct UpdateHistoryRow {
    pub id: String,
    pub version: String,
    pub channel: String,
    pub installed_at: i64,
    pub source: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

// Automatically implement Table trait from struct definition
impl_table_auto!(
    UpdateHistoryRow,
    "update_history",
    [version, channel, installed_at, source]
);

/// Data structure for UpdateHistoryRow operations (excludes id, created_at, updated_at)
#[derive(Debug, Clone)]
pub struct UpdateHistoryRowData {
    pub version: String,
    pub channel: String,
    pub installed_at: i64,
    pub source: Option<String>,
}

/// Insert a new UpdateHistoryRow record
/// Only data fields are required - id, created_at, and updated_at are set automatically
pub fn insert_one(data: UpdateHistoryRowData) -> Result<String> {
    let conn = db::get_connection()?;
    let row = UpdateHistoryRow {
        id: String::new(), // Set automatically
        version: data.version.clone(),
        channel: data.channel.clone(),
        installed_at: data.installed_at.clone(),
        source: data.source.clone(),

        created_at: 0, // Set automatically
        updated_at: 0, // Set automatically
    };
    DbTable::<UpdateHistoryRow>::insert(&conn, &row)
}

/// Insert multiple UpdateHistoryRow records
pub fn insert_many(data_vec: Vec<UpdateHistoryRowData>) -> Result<Vec<String>> {
    let conn = db::get_connection()?;
    let mut ids = Vec::new();
    for data in data_vec {
        let row = UpdateHistoryRow {
            id: String::new(), // Set automatically
            version: data.version.clone(),
            channel: data.channel.clone(),
            installed_at: data.installed_at.clone(),
            source: data.source.clone(),

            created_at: 0, // Set automatically
            updated_at: 0, // Set automatically
        };
        ids.push(DbTable::<UpdateHistoryRow>::insert(&conn, &row)?);
    }
    Ok(ids)
}

/// Upsert a UpdateHistoryRow record (insert if new, update if exists)
/// Only data fields are required - id, created_at, and updated_at are handled automatically
pub fn upsert_one(
    where_clause: &str,
    where_params: &[&dyn rusqlite::types::ToSql],
    data: UpdateHistoryRowData,
) -> Result<String> {
    let conn = db::get_connection()?;
    DbTable::<UpdateHistoryRow>::upsert_by(&conn, where_clause, where_params, |existing| {
        let mut row = existing.cloned().unwrap_or_else(|| {
            let mut r = UpdateHistoryRow {
                id: String::new(), // Set automatically
                version: String::new(),
                channel: String::new(),
                installed_at: 0,
                source: None,

                created_at: 0, // Set automatically
                updated_at: 0, // Set automatically
            };
            // Set initial values from data
            r.version = data.version.clone();
            r.channel = data.channel.clone();
            r.installed_at = data.installed_at.clone();
            r.source = data.source.clone();

            r
        });
        // Update only the data fields
        row.version = data.version;
        row.channel = data.channel;
        row.installed_at = data.installed_at;
        row.source = data.source;

        row
    })
}

/// Select one UpdateHistoryRow record
pub fn select_one(
    where_clause: &str,
    params: &[&dyn rusqlite::types::ToSql],
) -> Result<Option<UpdateHistoryRow>> {
    let conn = db::get_connection()?;
    DbTable::<UpdateHistoryRow>::select_one(&conn, where_clause, params)
}

/// Select many UpdateHistoryRow records
pub fn select_many(
    where_clause: &str,
    params: &[&dyn rusqlite::types::ToSql],
) -> Result<Vec<UpdateHistoryRow>> {
    let conn = db::get_connection()?;
    DbTable::<UpdateHistoryRow>::select_many(&conn, where_clause, params)
}

/// Delete UpdateHistoryRow record by primary key (id)
pub fn delete_by_id(id: &str) -> Result<usize> {
    let conn = db::get_connection()?;
    DbTable::<UpdateHistoryRow>::delete_many(
        &conn,
        "id = ?1",
        &[&id as &dyn rusqlite::types::ToSql],
    )
}

use chrono;

/// Record an update installation
pub fn record_update(version: &str, channel: &str, source: Option<&str>) -> Result<()> {
    insert_one(UpdateHistoryRowData {
        version: version.to_string(),
        channel: channel.to_string(),
        installed_at: chrono::Utc::now().timestamp(),
        source: source.map(|s| s.to_string()),
    })?;
    Ok(())
}

/// Get update history
pub fn get_update_history(
    limit: Option<i32>,
) -> Result<Vec<(String, String, i64, Option<String>)>> {
    let rows = select_many("1=1", &[])?;
    let mut sorted: Vec<_> = rows.into_iter().collect();
    sorted.sort_by(|a, b| b.installed_at.cmp(&a.installed_at));
    let limit = limit.unwrap_or(10) as usize;
    Ok(sorted
        .into_iter()
        .take(limit)
        .map(|r| (r.version, r.channel, r.installed_at, r.source))
        .collect())
}
