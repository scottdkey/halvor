use anyhow::Result;

/// Convert bytes to a trimmed string, handling UTF-8 conversion errors gracefully
pub fn bytes_to_string(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).trim().to_string()
}

/// Convert bytes to a string, returning an error if conversion fails
pub fn bytes_to_string_strict(bytes: &[u8]) -> Result<String> {
    Ok(String::from_utf8(bytes.to_vec())?.trim().to_string())
}

/// Format a host and port as an address string
pub fn format_address(host: &str, port: u16) -> String {
    format!("{}:{}", host, port)
}

/// Format a bind address (0.0.0.0:port)
pub fn format_bind_address(port: u16) -> String {
    format!("0.0.0.0:{}", port)
}
