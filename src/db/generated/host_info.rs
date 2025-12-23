// Auto-generated from database schema
// This file is generated - do not edit manually
// Run `halvor db generate` to regenerate

use crate::db;
use crate::db::core::table::DbTable;
use crate::impl_table_auto;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct HostInfoRow {
    pub id: String,
    pub hostname: Option<String>,
    pub last_provisioned_at: Option<i64>,
    pub docker_version: Option<String>,
    pub tailscale_installed: Option<i32>,
    pub portainer_installed: Option<i32>,
    pub metadata: Option<String>,
    pub ip: Option<String>,
    pub tailscale: Option<String>,
    pub backup_path: Option<String>,
    pub hostname_field: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

// Automatically implement Table trait from struct definition
impl_table_auto!(
    HostInfoRow,
    "host_info",
    [
        hostname,
        last_provisioned_at,
        docker_version,
        tailscale_installed,
        portainer_installed,
        metadata,
        ip,
        tailscale,
        backup_path,
        hostname_field
    ]
);

/// Data structure for HostInfoRow operations (excludes id, created_at, updated_at)
#[derive(Debug, Clone)]
pub struct HostInfoRowData {
    pub hostname: Option<String>,
    pub last_provisioned_at: Option<i64>,
    pub docker_version: Option<String>,
    pub tailscale_installed: Option<i32>,
    pub portainer_installed: Option<i32>,
    pub metadata: Option<String>,
    pub ip: Option<String>,
    pub tailscale: Option<String>,
    pub backup_path: Option<String>,
    pub hostname_field: Option<String>,
}

/// Insert a new HostInfoRow record
/// Only data fields are required - id, created_at, and updated_at are set automatically
pub fn insert_one(data: HostInfoRowData) -> Result<String> {
    let conn = db::get_connection()?;
    let row = HostInfoRow {
        id: String::new(), // Set automatically
        hostname: data.hostname.clone(),
        last_provisioned_at: data.last_provisioned_at.clone(),
        docker_version: data.docker_version.clone(),
        tailscale_installed: data.tailscale_installed.clone(),
        portainer_installed: data.portainer_installed.clone(),
        metadata: data.metadata.clone(),
        ip: data.ip.clone(),
        tailscale: data.tailscale.clone(),
        backup_path: data.backup_path.clone(),
        hostname_field: data.hostname_field.clone(),

        created_at: 0, // Set automatically
        updated_at: 0, // Set automatically
    };
    DbTable::<HostInfoRow>::insert(&conn, &row)
}

/// Insert multiple HostInfoRow records
pub fn insert_many(data_vec: Vec<HostInfoRowData>) -> Result<Vec<String>> {
    let conn = db::get_connection()?;
    let mut ids = Vec::new();
    for data in data_vec {
        let row = HostInfoRow {
            id: String::new(), // Set automatically
            hostname: data.hostname.clone(),
            last_provisioned_at: data.last_provisioned_at.clone(),
            docker_version: data.docker_version.clone(),
            tailscale_installed: data.tailscale_installed.clone(),
            portainer_installed: data.portainer_installed.clone(),
            metadata: data.metadata.clone(),
            ip: data.ip.clone(),
            tailscale: data.tailscale.clone(),
            backup_path: data.backup_path.clone(),
            hostname_field: data.hostname_field.clone(),

            created_at: 0, // Set automatically
            updated_at: 0, // Set automatically
        };
        ids.push(DbTable::<HostInfoRow>::insert(&conn, &row)?);
    }
    Ok(ids)
}

/// Upsert a HostInfoRow record (insert if new, update if exists)
/// Only data fields are required - id, created_at, and updated_at are handled automatically
pub fn upsert_one(
    where_clause: &str,
    where_params: &[&dyn rusqlite::types::ToSql],
    data: HostInfoRowData,
) -> Result<String> {
    let conn = db::get_connection()?;
    DbTable::<HostInfoRow>::upsert_by(&conn, where_clause, where_params, |existing| {
        let mut row = existing.cloned().unwrap_or_else(|| {
            let mut r = HostInfoRow {
                id: String::new(), // Set automatically
                hostname: None,
                last_provisioned_at: None,
                docker_version: None,
                tailscale_installed: None,
                portainer_installed: None,
                metadata: None,
                ip: None,
                tailscale: None,
                backup_path: None,
                hostname_field: None,

                created_at: 0, // Set automatically
                updated_at: 0, // Set automatically
            };
            // Set initial values from data
            r.hostname = data.hostname.clone();
            r.last_provisioned_at = data.last_provisioned_at.clone();
            r.docker_version = data.docker_version.clone();
            r.tailscale_installed = data.tailscale_installed.clone();
            r.portainer_installed = data.portainer_installed.clone();
            r.metadata = data.metadata.clone();
            r.ip = data.ip.clone();
            r.tailscale = data.tailscale.clone();
            r.backup_path = data.backup_path.clone();
            r.hostname_field = data.hostname_field.clone();

            r
        });
        // Update only the data fields
        row.hostname = data.hostname;
        row.last_provisioned_at = data.last_provisioned_at;
        row.docker_version = data.docker_version;
        row.tailscale_installed = data.tailscale_installed;
        row.portainer_installed = data.portainer_installed;
        row.metadata = data.metadata;
        row.ip = data.ip;
        row.tailscale = data.tailscale;
        row.backup_path = data.backup_path;
        row.hostname_field = data.hostname_field;

        row
    })
}

/// Select one HostInfoRow record
pub fn select_one(
    where_clause: &str,
    params: &[&dyn rusqlite::types::ToSql],
) -> Result<Option<HostInfoRow>> {
    let conn = db::get_connection()?;
    DbTable::<HostInfoRow>::select_one(&conn, where_clause, params)
}

/// Select many HostInfoRow records
pub fn select_many(
    where_clause: &str,
    params: &[&dyn rusqlite::types::ToSql],
) -> Result<Vec<HostInfoRow>> {
    let conn = db::get_connection()?;
    DbTable::<HostInfoRow>::select_many(&conn, where_clause, params)
}

/// Delete HostInfoRow record by primary key (id)
pub fn delete_by_id(id: &str) -> Result<usize> {
    let conn = db::get_connection()?;
    DbTable::<HostInfoRow>::delete_many(&conn, "id = ?1", &[&id as &dyn rusqlite::types::ToSql])
}

/// Delete HostInfoRow record by unique key: hostname
pub fn delete_by_hostname(hostname_value: &str) -> Result<usize> {
    let conn = db::get_connection()?;
    DbTable::<HostInfoRow>::delete_many(
        &conn,
        "hostname = ?1",
        &[&hostname_value as &dyn rusqlite::types::ToSql],
    )
}

use crate::config;
use chrono;

/// Store host provisioning information
pub fn store_host_info(
    hostname: &str,
    docker_version: Option<&str>,
    tailscale_installed: bool,
    portainer_installed: bool,
    metadata: Option<&str>,
) -> Result<()> {
    upsert_one(
        "hostname = ?1",
        &[&hostname as &dyn rusqlite::types::ToSql],
        HostInfoRowData {
            hostname: Some(hostname.to_string()),
            last_provisioned_at: Some(chrono::Utc::now().timestamp()),
            docker_version: docker_version.map(|s| s.to_string()),
            tailscale_installed: Some(tailscale_installed as i32),
            portainer_installed: Some(portainer_installed as i32),
            metadata: metadata.map(|s| s.to_string()),
            ip: None,
            hostname_field: None,
            tailscale: None,
            backup_path: None,
        },
    )?;
    Ok(())
}

/// Get host provisioning information
pub fn get_host_info(
    hostname: &str,
) -> Result<Option<(Option<i64>, Option<String>, bool, bool, Option<String>)>> {
    let row = select_one("hostname = ?1", &[&hostname as &dyn rusqlite::types::ToSql])?;
    Ok(row.map(|r| {
        (
            r.last_provisioned_at,
            r.docker_version,
            r.tailscale_installed.unwrap_or(0) != 0,
            r.portainer_installed.unwrap_or(0) != 0,
            r.metadata,
        )
    }))
}

/// List all known hosts
pub fn list_hosts() -> Result<Vec<String>> {
    let rows = select_many("1=1", &[])?;
    let mut hostnames: Vec<String> = rows.into_iter().filter_map(|r| r.hostname).collect();
    hostnames.sort();
    Ok(hostnames)
}

impl From<HostInfoRow> for config::HostConfig {
    fn from(row: HostInfoRow) -> Self {
        // Map tailscale column to hostname field (for backward compatibility)
        // Prefer hostname_field if set, otherwise use tailscale column
        config::HostConfig {
            ip: row.ip,
            hostname: row.hostname_field.or(row.tailscale),
            backup_path: row.backup_path,
        }
    }
}

/// Get host configuration from database
pub fn get_host_config(hostname: &str) -> Result<Option<config::HostConfig>> {
    let row = select_one("hostname = ?1", &[&hostname as &dyn rusqlite::types::ToSql])?;
    Ok(row.map(|r| r.into()))
}

/// Store host configuration in database
pub fn store_host_config(hostname: &str, config: &config::HostConfig) -> Result<()> {
    upsert_one(
        "hostname = ?1",
        &[&hostname as &dyn rusqlite::types::ToSql],
        HostInfoRowData {
            hostname: Some(hostname.to_string()),
            last_provisioned_at: Some(chrono::Utc::now().timestamp()),
            docker_version: None,
            tailscale_installed: Some(0),
            portainer_installed: Some(0),
            metadata: None,
            ip: config.ip.clone(),
            hostname_field: config.hostname.clone(),
            tailscale: config.hostname.clone(), // Map hostname to tailscale column for backward compatibility
            backup_path: config.backup_path.clone(),
        },
    )?;
    Ok(())
}

/// Delete host configuration from database
pub fn delete_host_config(hostname: &str) -> Result<()> {
    delete_by_hostname(hostname)?;
    Ok(())
}
