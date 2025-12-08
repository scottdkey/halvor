use crate::config_manager;
use crate::crypto;
use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use std::path::PathBuf;

const DB_FILE_NAME: &str = "hal.db";

/// Get the database file path (in the config directory)
pub fn get_db_path() -> Result<PathBuf> {
    let config_dir = config_manager::get_config_dir()?;
    Ok(config_dir.join(DB_FILE_NAME))
}

/// Initialize the database and create tables if they don't exist
pub fn init_db() -> Result<Connection> {
    let db_path = get_db_path()?;
    let conn = Connection::open(&db_path)
        .with_context(|| format!("Failed to open database: {}", db_path.display()))?;

    // Create tables
    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS update_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            version TEXT NOT NULL,
            channel TEXT NOT NULL,
            installed_at INTEGER NOT NULL,
            source TEXT
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS host_info (
            hostname TEXT PRIMARY KEY,
            last_provisioned_at INTEGER,
            docker_version TEXT,
            tailscale_installed INTEGER,
            portainer_installed INTEGER,
            metadata TEXT
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS encrypted_env_data (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            hostname TEXT,
            key TEXT NOT NULL,
            encrypted_value TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            UNIQUE(hostname, key)
        )",
        [],
    )?;

    Ok(conn)
}

/// Get a database connection
pub fn get_connection() -> Result<Connection> {
    init_db()
}

/// Set a setting value
pub fn set_setting(key: &str, value: &str) -> Result<()> {
    let conn = get_connection()?;
    let now = chrono::Utc::now().timestamp();

    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)",
        params![key, value, now],
    )?;

    Ok(())
}

/// Get a setting value
pub fn get_setting(key: &str) -> Result<Option<String>> {
    let conn = get_connection()?;

    let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
    let mut rows = stmt.query_map(params![key], |row| Ok(row.get::<_, String>(0)?))?;

    if let Some(row) = rows.next() {
        Ok(Some(row?))
    } else {
        Ok(None)
    }
}

/// Record an update installation
pub fn record_update(version: &str, channel: &str, source: Option<&str>) -> Result<()> {
    let conn = get_connection()?;
    let now = chrono::Utc::now().timestamp();

    conn.execute(
        "INSERT INTO update_history (version, channel, installed_at, source) VALUES (?1, ?2, ?3, ?4)",
        params![version, channel, now, source],
    )?;

    Ok(())
}

/// Get update history
pub fn get_update_history(
    limit: Option<i32>,
) -> Result<Vec<(String, String, i64, Option<String>)>> {
    let conn = get_connection()?;
    let limit = limit.unwrap_or(10);

    let mut stmt = conn.prepare(
        "SELECT version, channel, installed_at, source FROM update_history ORDER BY installed_at DESC LIMIT ?1"
    )?;

    let rows = stmt.query_map(params![limit], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, Option<String>>(3)?,
        ))
    })?;

    let mut history = Vec::new();
    for row in rows {
        history.push(row?);
    }

    Ok(history)
}

/// Store host information
pub fn store_host_info(
    hostname: &str,
    docker_version: Option<&str>,
    tailscale_installed: bool,
    portainer_installed: bool,
    metadata: Option<&str>,
) -> Result<()> {
    let conn = get_connection()?;
    let now = chrono::Utc::now().timestamp();

    conn.execute(
        "INSERT OR REPLACE INTO host_info (hostname, last_provisioned_at, docker_version, tailscale_installed, portainer_installed, metadata) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![hostname, now, docker_version, tailscale_installed as i32, portainer_installed as i32, metadata],
    )?;

    Ok(())
}

/// Get host information
pub fn get_host_info(
    hostname: &str,
) -> Result<Option<(Option<i64>, Option<String>, bool, bool, Option<String>)>> {
    let conn = get_connection()?;

    let mut stmt = conn.prepare(
        "SELECT last_provisioned_at, docker_version, tailscale_installed, portainer_installed, metadata FROM host_info WHERE hostname = ?1"
    )?;

    let mut rows = stmt.query_map(params![hostname], |row| {
        Ok((
            row.get::<_, Option<i64>>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, i32>(2)? != 0,
            row.get::<_, i32>(3)? != 0,
            row.get::<_, Option<String>>(4)?,
        ))
    })?;

    if let Some(row) = rows.next() {
        Ok(Some(row?))
    } else {
        Ok(None)
    }
}

/// List all known hosts
pub fn list_hosts() -> Result<Vec<String>> {
    let conn = get_connection()?;

    let mut stmt = conn.prepare("SELECT hostname FROM host_info ORDER BY hostname")?;
    let rows = stmt.query_map([], |row| Ok(row.get::<_, String>(0)?))?;

    let mut hosts = Vec::new();
    for row in rows {
        hosts.push(row?);
    }

    Ok(hosts)
}

/// Store encrypted environment variable
pub fn store_encrypted_env(hostname: Option<&str>, key: &str, value: &str) -> Result<()> {
    let conn = get_connection()?;
    let now = chrono::Utc::now().timestamp();

    let encrypted = crypto::encrypt(value)?;

    conn.execute(
        "INSERT OR REPLACE INTO encrypted_env_data (hostname, key, encrypted_value, created_at, updated_at) 
         VALUES (?1, ?2, ?3, COALESCE((SELECT created_at FROM encrypted_env_data WHERE hostname IS ?1 AND key = ?2), ?4), ?4)",
        params![hostname, key, encrypted, now],
    )?;

    Ok(())
}

/// Get encrypted environment variable
pub fn get_encrypted_env(hostname: Option<&str>, key: &str) -> Result<Option<String>> {
    let conn = get_connection()?;

    let mut stmt = conn.prepare(
        "SELECT encrypted_value FROM encrypted_env_data WHERE hostname IS ?1 AND key = ?2",
    )?;

    let mut rows = stmt.query_map(params![hostname, key], |row| Ok(row.get::<_, String>(0)?))?;

    if let Some(row) = rows.next() {
        let encrypted = row?;
        let decrypted = crypto::decrypt(&encrypted)?;
        Ok(Some(decrypted))
    } else {
        Ok(None)
    }
}

/// Get all encrypted environment variables for a hostname (or global if None)
pub fn get_all_encrypted_envs(hostname: Option<&str>) -> Result<Vec<(String, String)>> {
    let conn = get_connection()?;

    let mut stmt = conn.prepare(
        "SELECT key, encrypted_value FROM encrypted_env_data WHERE hostname IS ?1 ORDER BY key",
    )?;

    let rows = stmt.query_map(params![hostname], |row| {
        let key: String = row.get(0)?;
        let encrypted: String = row.get(1)?;
        Ok((key, encrypted))
    })?;

    // Decrypt after retrieving from database
    let mut envs = Vec::new();
    for row in rows {
        let (key, encrypted) = row?;
        match crypto::decrypt(&encrypted) {
            Ok(decrypted) => envs.push((key, decrypted)),
            Err(e) => {
                eprintln!("Warning: Failed to decrypt {}: {}", key, e);
                continue;
            }
        }
    }

    Ok(envs)
}

/// Export all encrypted data for syncing
pub fn export_encrypted_data() -> Result<Vec<u8>> {
    let conn = get_connection()?;

    let mut stmt = conn.prepare(
        "SELECT hostname, key, encrypted_value FROM encrypted_env_data ORDER BY hostname, key",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, Option<String>>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;

    let mut data = Vec::new();
    for row in rows {
        let (hostname, key, encrypted_value) = row?;
        data.push((hostname, key, encrypted_value));
    }

    // Serialize to JSON
    let json =
        serde_json::to_string(&data).context("Failed to serialize encrypted data to JSON")?;
    Ok(json.into_bytes())
}

/// Import encrypted data from sync
pub fn import_encrypted_data(data: &[u8]) -> Result<()> {
    let conn = get_connection()?;
    let now = chrono::Utc::now().timestamp();

    let data: Vec<(Option<String>, String, String)> =
        serde_json::from_slice(data).context("Failed to parse encrypted data")?;

    for (hostname, key, encrypted_value) in data {
        conn.execute(
            "INSERT OR REPLACE INTO encrypted_env_data (hostname, key, encrypted_value, created_at, updated_at) 
             VALUES (?1, ?2, ?3, COALESCE((SELECT created_at FROM encrypted_env_data WHERE hostname IS ?1 AND key = ?2), ?4), ?4)",
            params![hostname, key, encrypted_value, now],
        )?;
    }

    Ok(())
}
