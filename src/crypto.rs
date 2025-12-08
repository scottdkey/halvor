use crate::config_manager;
use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose};
use std::fs;
use std::path::PathBuf;

const KEY_FILE_NAME: &str = ".hal_key";

/// Get or create the encryption key
fn get_or_create_key() -> Result<Key<Aes256Gcm>> {
    let config_dir = config_manager::get_config_dir()?;
    let key_path = config_dir.join(KEY_FILE_NAME);

    let key = if key_path.exists() {
        // Load existing key
        let key_bytes = fs::read(&key_path)
            .with_context(|| format!("Failed to read key file: {}", key_path.display()))?;
        if key_bytes.len() != 32 {
            anyhow::bail!("Invalid key file: wrong length");
        }
        *Key::<Aes256Gcm>::from_slice(&key_bytes)
    } else {
        // Generate new key
        let key = Aes256Gcm::generate_key(&mut OsRng);
        fs::write(&key_path, key.as_slice())
            .with_context(|| format!("Failed to write key file: {}", key_path.display()))?;
        // Set restrictive permissions (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600))
                .with_context(|| format!("Failed to set key file permissions"))?;
        }
        key
    };

    Ok(key)
}

/// Encrypt data
pub fn encrypt(data: &str) -> Result<String> {
    let key = get_or_create_key()?;
    let cipher = Aes256Gcm::new(&key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, data.as_bytes())
        .map_err(|e| anyhow::anyhow!("Failed to encrypt data: {}", e))?;

    // Combine nonce and ciphertext, then base64 encode
    let mut combined = nonce.to_vec();
    combined.extend_from_slice(&ciphertext);
    Ok(general_purpose::STANDARD.encode(&combined))
}

/// Decrypt data
pub fn decrypt(encrypted: &str) -> Result<String> {
    let key = get_or_create_key()?;
    let cipher = Aes256Gcm::new(&key);

    // Decode from base64
    let combined = general_purpose::STANDARD
        .decode(encrypted)
        .context("Failed to decode base64")?;

    if combined.len() < 12 {
        anyhow::bail!("Invalid encrypted data: too short");
    }

    // Extract nonce (first 12 bytes) and ciphertext (rest)
    let nonce = Nonce::from_slice(&combined[0..12]);
    let ciphertext = &combined[12..];

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow::anyhow!("Failed to decrypt data: {}", e))?;

    String::from_utf8(plaintext).context("Failed to convert decrypted data to string")
}

/// Export the encryption key (for syncing to another machine)
pub fn export_key() -> Result<String> {
    let config_dir = config_manager::get_config_dir()?;
    let key_path = config_dir.join(KEY_FILE_NAME);

    if !key_path.exists() {
        anyhow::bail!("Encryption key not found. Generate one by encrypting data first.");
    }

    let key_bytes = fs::read(&key_path)
        .with_context(|| format!("Failed to read key file: {}", key_path.display()))?;

    Ok(general_purpose::STANDARD.encode(&key_bytes))
}

/// Import an encryption key (for syncing from another machine)
pub fn import_key(key_base64: &str) -> Result<()> {
    let config_dir = config_manager::get_config_dir()?;
    let key_path = config_dir.join(KEY_FILE_NAME);

    if key_path.exists() {
        anyhow::bail!(
            "Encryption key already exists. Remove it first if you want to import a new one."
        );
    }

    let key_bytes = general_purpose::STANDARD
        .decode(key_base64)
        .context("Failed to decode key")?;

    if key_bytes.len() != 32 {
        anyhow::bail!("Invalid key: wrong length");
    }

    fs::write(&key_path, &key_bytes)
        .with_context(|| format!("Failed to write key file: {}", key_path.display()))?;

    // Set restrictive permissions (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600))
            .with_context(|| format!("Failed to set key file permissions"))?;
    }

    Ok(())
}

/// Check if encryption key exists
pub fn key_exists() -> Result<bool> {
    let config_dir = config_manager::get_config_dir()?;
    let key_path = config_dir.join(KEY_FILE_NAME);
    Ok(key_path.exists())
}
