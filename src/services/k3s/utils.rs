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

/// Parse K3s node-token file content and extract the token
/// 
/// K3s node-token files have the format: `K<node-id>::server:<token>`
/// This function extracts just the token part (after `::server:`) for use in join commands
/// and storage in 1Password.
/// 
/// If the input is already just a token (no `::server:`), it returns it as-is.
pub fn parse_node_token(token_content: &str) -> String {
    let trimmed = token_content.trim();
    
    // Check if it's in the full format: K<node-id>::server:<token>
    if let Some(server_pos) = trimmed.find("::server:") {
        // Extract the token part (everything after "::server:")
        trimmed[server_pos + 9..].trim().to_string()
    } else {
        // Already just a token, return as-is
        trimmed.to_string()
    }
}

/// Check if we're in development mode
pub fn is_development_mode() -> bool {
    std::env::var("HALVOR_ENV")
        .map(|v| v.to_lowercase() == "development")
        .unwrap_or(false)
}
