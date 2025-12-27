//! Agent mesh security - join tokens and peer key management

use crate::db;
use crate::db::generated::{agent_peers, join_tokens, peer_keys};
use crate::db::generated::{AgentPeersRowData, JoinTokensRowData, PeerKeysRowData};
use crate::utils::crypto;
use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const TOKEN_EXPIRY_HOURS: i64 = 24;

/// Join token structure (encoded in base64)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinToken {
    pub token_id: String,
    pub issuer_hostname: String,
    pub issuer_ip: String,
    pub issuer_port: u16,
    pub expires_at: i64,
    /// Encrypted shared secret for initial handshake
    pub handshake_key: String,
}

impl JoinToken {
    /// Encode token as base64 string
    pub fn encode(&self) -> Result<String> {
        let json = serde_json::to_string(self)?;
        Ok(general_purpose::STANDARD.encode(json.as_bytes()))
    }

    /// Decode token from base64 string
    pub fn decode(encoded: &str) -> Result<Self> {
        let bytes = general_purpose::STANDARD
            .decode(encoded)
            .context("Failed to decode base64 token")?;
        let json = String::from_utf8(bytes).context("Invalid UTF-8 in token")?;
        let token: JoinToken = serde_json::from_str(&json).context("Invalid token format")?;
        Ok(token)
    }

    /// Check if token has expired
    pub fn is_expired(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        now > self.expires_at
    }
}

/// Generate a join token for a new agent to join the mesh
pub fn generate_join_token(
    issuer_hostname: &str,
    issuer_ip: &str,
    issuer_port: u16,
) -> Result<(String, JoinToken)> {
    let token_id = Uuid::new_v4().to_string();
    let expires_at = chrono::Utc::now().timestamp() + (TOKEN_EXPIRY_HOURS * 3600);

    // Generate a random handshake key (32 bytes for AES-256)
    let handshake_key = crypto::generate_random_key()?;
    let handshake_key_b64 = general_purpose::STANDARD.encode(&handshake_key);

    let token = JoinToken {
        token_id: token_id.clone(),
        issuer_hostname: issuer_hostname.to_string(),
        issuer_ip: issuer_ip.to_string(),
        issuer_port,
        expires_at,
        handshake_key: handshake_key_b64,
    };

    let encoded = token.encode()?;

    // Store token in database
    let data = JoinTokensRowData {
        token: encoded.clone(),
        issuer_hostname: issuer_hostname.to_string(),
        expires_at,
        used: 0,
        used_by_hostname: None,
        used_at: None,
    };

    join_tokens::insert_one(data)?;

    Ok((encoded, token))
}

/// Validate a join token
pub fn validate_join_token(encoded_token: &str) -> Result<JoinToken> {
    let token = JoinToken::decode(encoded_token)?;

    if token.is_expired() {
        anyhow::bail!("Join token has expired");
    }

    // Check if token exists in database and hasn't been used
    eprintln!("[DEBUG] Validating token_id: {}", token.token_id);
    eprintln!("[DEBUG] Searching for encoded token in database (first 50 chars): {}", &encoded_token[..50.min(encoded_token.len())]);

    let rows = join_tokens::select_many(
        "token = ?1 AND used = 0",
        &[&encoded_token as &dyn rusqlite::types::ToSql],
    )?;

    eprintln!("[DEBUG] Found {} matching tokens", rows.len());

    // Also check if token exists but was already used
    let all_rows = join_tokens::select_many(
        "token = ?1",
        &[&encoded_token as &dyn rusqlite::types::ToSql],
    )?;

    eprintln!("[DEBUG] Found {} total tokens (including used)", all_rows.len());
    if !all_rows.is_empty() {
        eprintln!("[DEBUG] Token exists - used={}", all_rows[0].used);
    }

    // List all tokens for debugging
    let all_tokens = join_tokens::select_many("1=1", &[])?;
    eprintln!("[DEBUG] Total tokens in database: {}", all_tokens.len());
    for (i, t) in all_tokens.iter().enumerate() {
        eprintln!("[DEBUG]   Token {}: issuer={}, used={}, token_preview={}",
            i, t.issuer_hostname, t.used, &t.token[..50.min(t.token.len())]);
    }

    if rows.is_empty() {
        anyhow::bail!("Invalid or already used join token");
    }

    Ok(token)
}

/// Mark a join token as used
pub fn mark_token_used(encoded_token: &str, joined_hostname: &str) -> Result<()> {
    let conn = db::get_connection()?;
    let now = chrono::Utc::now().timestamp();

    conn.execute(
        "UPDATE join_tokens SET used = 1, used_by_hostname = ?1, used_at = ?2 WHERE token = ?3",
        rusqlite::params![joined_hostname, now, encoded_token],
    )?;

    Ok(())
}

/// Add a peer to the mesh (called after successful join handshake)
pub fn add_peer(
    hostname: &str,
    tailscale_ip: Option<String>,
    tailscale_hostname: Option<String>,
    public_key: &str,
    shared_secret: &str,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();

    // Add to agent_peers table
    let peer_data = AgentPeersRowData {
        hostname: hostname.to_string(),
        tailscale_ip,
        tailscale_hostname,
        public_key: public_key.to_string(),
        status: "active".to_string(),
        last_seen_at: Some(now),
        joined_at: now,
    };

    agent_peers::upsert_one(
        "hostname = ?1",
        &[&hostname as &dyn rusqlite::types::ToSql],
        peer_data,
    )?;

    // Store shared secret in peer_keys table
    let key_data = PeerKeysRowData {
        peer_hostname: hostname.to_string(),
        shared_secret: shared_secret.to_string(),
        algorithm: "aes-256-gcm".to_string(),
    };

    peer_keys::upsert_one(
        "peer_hostname = ?1",
        &[&hostname as &dyn rusqlite::types::ToSql],
        key_data,
    )?;

    Ok(())
}

/// Get all active peers in the mesh
pub fn get_active_peers() -> Result<Vec<String>> {
    let rows = agent_peers::select_many(
        "status = ?1",
        &[&"active" as &dyn rusqlite::types::ToSql],
    )?;

    Ok(rows.into_iter().map(|r| r.hostname).collect())
}

/// Get shared secret for a peer
pub fn get_peer_shared_secret(peer_hostname: &str) -> Result<Option<String>> {
    let rows = peer_keys::select_many(
        "peer_hostname = ?1",
        &[&peer_hostname as &dyn rusqlite::types::ToSql],
    )?;

    Ok(rows.first().map(|r| r.shared_secret.clone()))
}

/// Update peer last seen timestamp
pub fn update_peer_last_seen(hostname: &str) -> Result<()> {
    let conn = db::get_connection()?;
    let now = chrono::Utc::now().timestamp();

    conn.execute(
        "UPDATE agent_peers SET last_seen_at = ?1 WHERE hostname = ?2",
        rusqlite::params![now, hostname],
    )?;

    Ok(())
}

/// Remove a peer from the mesh
pub fn remove_peer(hostname: &str) -> Result<()> {
    agent_peers::delete_by_hostname(hostname)?;
    // peer_keys will be deleted automatically via CASCADE
    Ok(())
}

/// Clean up expired join tokens
pub fn cleanup_expired_tokens() -> Result<usize> {
    let conn = db::get_connection()?;
    let now = chrono::Utc::now().timestamp();

    let deleted = conn.execute(
        "DELETE FROM join_tokens WHERE expires_at < ?1",
        rusqlite::params![now],
    )?;

    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_encode_decode() {
        let token = JoinToken {
            token_id: "test-123".to_string(),
            issuer_hostname: "frigg".to_string(),
            issuer_ip: "100.66.176.17".to_string(),
            issuer_port: 13500,
            expires_at: chrono::Utc::now().timestamp() + 3600,
            handshake_key: "test-key".to_string(),
        };

        let encoded = token.encode().unwrap();
        let decoded = JoinToken::decode(&encoded).unwrap();

        assert_eq!(token.token_id, decoded.token_id);
        assert_eq!(token.issuer_hostname, decoded.issuer_hostname);
        assert!(!decoded.is_expired());
    }
}
