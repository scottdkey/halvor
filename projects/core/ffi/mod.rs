// FFI module for multi-platform bindings
// This module contains platform-agnostic FFI code that will be used to generate
// platform-specific bindings (Swift, Kotlin, WASM)
//
// Functions in this module use existing types from the main crate:
// - crate::agent::client::HalvorClient
// - crate::agent::discovery::DiscoveredHost
// - crate::agent::server::HostInfo
//
// The build script (build.rs) automatically generates platform-specific bindings
// from functions marked with export macros.

// C FFI bindings for Swift (only compiled for non-WASM targets)
#[cfg(not(target_arch = "wasm32"))]
pub mod c_ffi;

// Re-export for convenience
pub use crate::agent::client::HalvorClient;
pub use crate::agent::discovery::DiscoveredHost;
pub use crate::agent::server::HostInfo;
