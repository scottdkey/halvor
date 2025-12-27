// FFI module for multi-platform bindings
// This module contains platform-agnostic FFI code that will be used to generate
// platform-specific bindings (Swift, Kotlin, WASM)

// C FFI bindings for Swift (only compiled for non-WASM targets)
#[cfg(not(target_arch = "wasm32"))]
pub mod c_ffi;

// Re-export for convenience
pub use crate::agent::client::HalvorClient;
pub use crate::agent::discovery::DiscoveredHost;
pub use crate::agent::server::HostInfo;

