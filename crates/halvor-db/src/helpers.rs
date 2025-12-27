//! Database helper functions that depend on config types
//! These provide high-level helpers that work with halvor-core config types

use anyhow::Result;
use halvor_core::config;
use halvor_core::utils::crypto;

// SMB Server helpers
pub fn store_smb_server(server_name: &str, smb_config: &config::SmbServerConfig) -> Result<()> {
    let shares_json = serde_json::to_string(&smb_config.shares)?;
    crate::smb_servers::upsert_one(
        "server_name = ?1",
        &[&server_name as &dyn rusqlite::types::ToSql],
        crate::smb_servers::SmbServersRowData {
            server_name: Some(server_name.to_string()),
            host: smb_config.host.clone(),
            shares: shares_json,
            username: smb_config.username.clone(),
            password: smb_config.password.clone(),
            options: smb_config.options.clone(),
        },
    )?;
    Ok(())
}

pub fn get_smb_server(server_name: &str) -> Result<Option<config::SmbServerConfig>> {
    let row = crate::smb_servers::select_one(
        "server_name = ?1",
        &[&server_name as &dyn rusqlite::types::ToSql],
    )?;
    Ok(row.map(|row| {
        let shares: Vec<String> = serde_json::from_str(&row.shares).unwrap_or_else(|_| Vec::new());
        config::SmbServerConfig {
            host: row.host,
            shares,
            username: row.username,
            password: row.password,
            options: row.options,
        }
    }))
}

pub fn delete_smb_server(server_name: &str) -> Result<()> {
    crate::smb_servers::delete_by_server_name(server_name)?;
    Ok(())
}

// Host Config helpers
pub fn get_host_config(hostname: &str) -> Result<Option<config::HostConfig>> {
    let row = crate::host_info::select_one("hostname = ?1", &[&hostname as &dyn rusqlite::types::ToSql])?;
    Ok(row.map(|r| config::HostConfig {
        ip: r.ip,
        hostname: r.hostname_field.or(r.tailscale),
        backup_path: r.backup_path,
        sudo_password: None,
        sudo_user: None,
    }))
}

pub fn store_host_config(hostname: &str, config: &config::HostConfig) -> Result<()> {
    crate::host_info::upsert_one(
        "hostname = ?1",
        &[&hostname as &dyn rusqlite::types::ToSql],
        crate::host_info::HostInfoRowData {
            hostname: Some(hostname.to_string()),
            last_provisioned_at: Some(chrono::Utc::now().timestamp()),
            docker_version: None,
            tailscale_installed: Some(0),
            portainer_installed: Some(0),
            metadata: None,
            ip: config.ip.clone(),
            hostname_field: config.hostname.clone(),
            tailscale: config.hostname.clone(),
            backup_path: config.backup_path.clone(),
        },
    )?;
    Ok(())
}

pub fn delete_host_config(hostname: &str) -> Result<()> {
    crate::host_info::delete_by_hostname(hostname)?;
    Ok(())
}

// Encrypted env helpers
pub fn store_encrypted_env(hostname: Option<&str>, key: &str, value: &str) -> Result<()> {
    let encrypted = crypto::encrypt(value)?;
    crate::encrypted_env_data::upsert_one(
        "hostname IS ?1 AND key = ?2",
        &[
            &hostname as &dyn rusqlite::types::ToSql,
            &key as &dyn rusqlite::types::ToSql,
        ],
        crate::encrypted_env_data::EncryptedEnvDataRowData {
            hostname: hostname.map(|s| s.to_string()),
            key: key.to_string(),
            encrypted_value: encrypted,
        },
    )?;
    Ok(())
}

pub fn get_encrypted_env(hostname: Option<&str>, key: &str) -> Result<Option<String>> {
    let row = crate::encrypted_env_data::select_one(
        "hostname IS ?1 AND key = ?2",
        &[
            &hostname as &dyn rusqlite::types::ToSql,
            &key as &dyn rusqlite::types::ToSql,
        ],
    )?;
    Ok(row.and_then(|r| crypto::decrypt(&r.encrypted_value).ok()))
}

pub fn get_all_encrypted_envs(hostname: Option<&str>) -> Result<Vec<(String, String)>> {
    let rows = crate::encrypted_env_data::select_many(
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

