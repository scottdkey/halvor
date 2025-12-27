// Utils module - common code that calls outside of other modules
pub mod crypto;
pub mod env;
pub mod exec;
// Note: ffi_bindings moved to halvor-cli (depends on syn/quote)
pub mod hostname;  // Hostname utilities (extracted from config::service)
pub mod json_stream;
pub mod networking;
// Note: service module moved to halvor-cli (depends on halvor_docker)
pub mod ssh;
pub mod string;
pub mod update;

// Re-export commonly used utilities
pub use json_stream::{read_json, send_json_request, write_json};
pub use string::{bytes_to_string, bytes_to_string_strict, format_address, format_bind_address};
