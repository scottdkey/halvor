//! K3s utility functions

use anyhow::Result;
use rand::RngCore;

/// Generate a random hex token (64 characters = 32 bytes)
/// Uses native Rust rand crate for reliable cross-platform token generation
pub fn generate_cluster_token() -> Result<String> {
    let mut rng = rand::thread_rng();
    let mut token = String::with_capacity(64);
    for _ in 0..64 {
        let mut bytes = [0u8; 1];
        rng.fill_bytes(&mut bytes);
        token.push_str(&format!("{:x}", bytes[0]));
    }

    Ok(token)
}

/// Check if we're in development mode
pub fn is_development_mode() -> bool {
    std::env::var("HALVOR_ENV")
        .map(|v| v.to_lowercase() == "development")
        .unwrap_or(false)
}
